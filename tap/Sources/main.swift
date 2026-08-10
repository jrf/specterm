import Foundation
import ScreenCaptureKit
import CoreMedia
import AVFoundation
import AppKit
import SpectermTapCore

// specterm-tap: Captures system audio via ScreenCaptureKit and writes raw f32
// PCM samples to stdout (native endian). Sends interleaved stereo (L, R, L, R...)
// by default, or mono with --mono. Requires macOS 13+.

func log(_ msg: String) {
    FileHandle.standardError.write("\(msg)\n".data(using: .utf8)!)
}

// Keep strong references so they aren't deallocated
var globalStream: SCStream?
var globalTap: AudioTap?
var globalSigSrc: DispatchSourceSignal?
var globalTermSrc: DispatchSourceSignal?
var globalWatchdog: DispatchSourceTimer?
var globalSleepObs: NSObjectProtocol?

@available(macOS 13.0, *)
func stopCaptureBeforeSleep(timeout: TimeInterval = 10) {
    guard let stream = globalStream else { return }

    let stopped = DispatchSemaphore(value: 0)
    stream.stopCapture { error in
        if let error {
            log("failed to stop stream before sleep: \(error.localizedDescription)")
        }
        stopped.signal()
    }

    if stopped.wait(timeout: .now() + timeout) == .timedOut {
        log("timed out waiting for stream to stop before sleep")
    }

    globalStream = nil
    globalTap = nil
}

@available(macOS 13.0, *)
class AudioTap: NSObject, SCStreamOutput, SCStreamDelegate {
    let outputHandle = FileHandle.standardOutput
    let monoMode: Bool
    var samplesReceived: UInt64 = 0

    init(mono: Bool = false) {
        self.monoMode = mono
        super.init()
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .audio else { return }
        guard CMSampleBufferDataIsReady(sampleBuffer) else { return }

        // ScreenCaptureKit delivers audio as non-interleaved (planar) Float32 —
        // channels come as separate buffers in the AudioBufferList. The old
        // CMSampleBufferGetDataBuffer path returned only channel 0, so the Rust
        // side saw the same signal on both "L" and "R". We fetch the full ABL
        // and normalize to per-channel arrays regardless of layout.
        guard let formatDesc = CMSampleBufferGetFormatDescription(sampleBuffer),
              let asbdPtr = CMAudioFormatDescriptionGetStreamBasicDescription(formatDesc) else {
            return
        }
        let asbd = asbdPtr.pointee
        let channels = Int(asbd.mChannelsPerFrame)
        let isPlanar = (asbd.mFormatFlags & AudioFormatFlags(kAudioFormatFlagIsNonInterleaved)) != 0

        guard let channelData = extractChannels(from: sampleBuffer, channels: channels, isPlanar: isPlanar),
              let framesPerChannel = channelData.first?.count,
              framesPerChannel > 0 else {
            return
        }

        let output: [Float]
        if monoMode {
            var mono = [Float](repeating: 0, count: framesPerChannel)
            for ch in channelData {
                let n = min(ch.count, framesPerChannel)
                for i in 0..<n { mono[i] += ch[i] }
            }
            let denom = Float(channelData.count)
            if denom > 1 {
                for i in 0..<framesPerChannel { mono[i] /= denom }
            }
            output = mono
        } else {
            // Interleaved stereo (L,R,L,R...) from channels 0 and 1.
            // Mono source: duplicate.
            let left = channelData[0]
            let right = channelData.count > 1 ? channelData[1] : channelData[0]
            var stereo: [Float] = []
            stereo.reserveCapacity(framesPerChannel * 2)
            for i in 0..<framesPerChannel {
                stereo.append(left[i])
                stereo.append(i < right.count ? right[i] : 0)
            }
            output = stereo
        }

        output.withUnsafeBufferPointer { buf in
            outputHandle.write(Data(UnsafeRawBufferPointer(buf)))
        }

        if samplesReceived == 0 {
            let layout = isPlanar ? "planar" : "interleaved"
            log("receiving audio (\(channels)ch \(layout) → \(monoMode ? "mono" : "stereo"), \(framesPerChannel) frames)")
        }
        samplesReceived += UInt64(output.count)
    }

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        log("stream stopped with error: \(error.localizedDescription)")
        exit(1)
    }

    /// Pull per-channel Float32 arrays from a CMSampleBuffer regardless of
    /// whether the audio is delivered interleaved or as non-interleaved planes.
    private func extractChannels(from sampleBuffer: CMSampleBuffer, channels: Int, isPlanar: Bool) -> [[Float]]? {
        // AudioBufferList is variable-length — query the required size first.
        var sizeNeeded = 0
        var status = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
            sampleBuffer,
            bufferListSizeNeededOut: &sizeNeeded,
            bufferListOut: nil,
            bufferListSize: 0,
            blockBufferAllocator: nil,
            blockBufferMemoryAllocator: nil,
            flags: 0,
            blockBufferOut: nil
        )
        guard status == noErr, sizeNeeded > 0 else { return nil }

        let ablRaw = UnsafeMutableRawPointer.allocate(
            byteCount: sizeNeeded,
            alignment: MemoryLayout<AudioBufferList>.alignment
        )
        defer { ablRaw.deallocate() }
        let ablPtr = ablRaw.assumingMemoryBound(to: AudioBufferList.self)

        var blockBuffer: CMBlockBuffer?
        status = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
            sampleBuffer,
            bufferListSizeNeededOut: nil,
            bufferListOut: ablPtr,
            bufferListSize: sizeNeeded,
            blockBufferAllocator: nil,
            blockBufferMemoryAllocator: nil,
            flags: 0,
            blockBufferOut: &blockBuffer
        )
        guard status == noErr else { return nil }

        let bufferList = UnsafeMutableAudioBufferListPointer(ablPtr)

        if isPlanar {
            var out: [[Float]] = []
            out.reserveCapacity(bufferList.count)
            for buf in bufferList {
                guard let ptr = buf.mData?.assumingMemoryBound(to: Float.self) else { continue }
                let count = Int(buf.mDataByteSize) / MemoryLayout<Float>.size
                out.append(Array(UnsafeBufferPointer(start: ptr, count: count)))
            }
            return out.isEmpty ? nil : out
        } else {
            guard bufferList.count > 0,
                  let ptr = bufferList[0].mData?.assumingMemoryBound(to: Float.self) else {
                return nil
            }
            let total = Int(bufferList[0].mDataByteSize) / MemoryLayout<Float>.size
            let frames = channels > 0 ? total / channels : 0
            guard frames > 0 else { return nil }
            var out = [[Float]](repeating: [Float](repeating: 0, count: frames), count: channels)
            for f in 0..<frames {
                for ch in 0..<channels {
                    out[ch][f] = ptr[f * channels + ch]
                }
            }
            return out
        }
    }
}

@available(macOS 13.0, *)
func setup() async throws {
    let args = CommandLine.arguments
    let sampleRate = sampleRate(from: args)

    log("starting capture (sample rate: \(Int(sampleRate)))")

    let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)

    guard let display = content.displays.first else {
        log("error: no displays found")
        exit(1)
    }

    log("display: \(display.width)x\(display.height)")

    let filter = SCContentFilter(display: display, excludingApplications: [], exceptingWindows: [])

    let config = SCStreamConfiguration()
    config.capturesAudio = true
    config.excludesCurrentProcessAudio = true
    // Minimize video overhead — we only need audio
    config.showsCursor = false
    config.width = 2
    config.height = 2
    config.minimumFrameInterval = CMTime(value: 1, timescale: 1)
    config.sampleRate = Int(sampleRate)
    config.channelCount = 2

    let monoMode = args.contains("--mono")
    let tap = AudioTap(mono: monoMode)
    let stream = SCStream(filter: filter, configuration: config, delegate: tap)

    try stream.addStreamOutput(tap, type: .audio, sampleHandlerQueue: .global(qos: .userInteractive))

    // Store in globals so they aren't deallocated
    globalTap = tap
    globalStream = stream

    log("starting stream...")
    try await stream.startCapture()
    log("stream started, waiting for audio...")

    // Signal handlers on main queue
    let sigSrc = DispatchSource.makeSignalSource(signal: SIGINT, queue: .main)
    signal(SIGINT, SIG_IGN)
    sigSrc.setEventHandler {
        log("received SIGINT, stopping...")
        Task {
            try? await globalStream?.stopCapture()
            exit(0)
        }
    }
    sigSrc.resume()
    globalSigSrc = sigSrc

    let termSrc = DispatchSource.makeSignalSource(signal: SIGTERM, queue: .main)
    signal(SIGTERM, SIG_IGN)
    termSrc.setEventHandler {
        log("received SIGTERM, stopping...")
        Task {
            try? await globalStream?.stopCapture()
            exit(0)
        }
    }
    termSrc.resume()
    globalTermSrc = termSrc

    let parentPid = getppid()

    // Shut down the entire application before sleep. replayd can otherwise hold
    // a zombie SCStream reference across sleep/wake, stranding the
    // screen-recording indicator and sending the daemon into a CPU-spin.
    //
    // Signal the parent before exiting so specterm's tap health checker cannot
    // interpret our clean exit as a crash and respawn capture after wake.
    globalSleepObs = NSWorkspace.shared.notificationCenter.addObserver(
        forName: NSWorkspace.willSleepNotification,
        object: nil,
        queue: .main
    ) { _ in
        log("system will sleep, stopping specterm...")
        stopCaptureBeforeSleep()
        kill(parentPid, SIGTERM)
        exit(0)
    }

    // Watchdog: exit if parent process dies (reparented to launchd, ppid == 1).
    // This prevents orphaned tap processes after sleep/wake or unclean shutdown.
    let watchdog = DispatchSource.makeTimerSource(queue: .global(qos: .utility))
    watchdog.schedule(deadline: .now() + 2, repeating: 2.0)
    watchdog.setEventHandler {
        if getppid() != parentPid {
            log("parent process died, stopping...")
            Task {
                try? await globalStream?.stopCapture()
                exit(0)
            }
        }
    }
    watchdog.resume()
    globalWatchdog = watchdog

    // setup() returns — dispatchMain() in the entry point keeps the process alive
}

if #available(macOS 13.0, *) {
    Task {
        do {
            try await setup()
        } catch {
            log("error: \(error.localizedDescription)")
            exit(1)
        }
    }
    // Single dispatchMain() on the actual main thread — keeps the process alive
    // and services signal handler dispatch sources
    dispatchMain()
} else {
    log("error: requires macOS 13.0 or later")
    exit(1)
}
