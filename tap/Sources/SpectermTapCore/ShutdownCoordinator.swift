import Foundation

/// Serializes shutdown requests without blocking the caller while capture stops.
/// Callbacks run outside the lock and may arrive synchronously or on any queue.
/// Injected operations must therefore be safe on the requesting/callback queues.
/// Mutable coordination state is protected by `lock`; dependencies are immutable.
public final class ShutdownCoordinator: @unchecked Sendable {
    public enum Reason: Sendable {
        case signal, sleep, parentExited, captureError
    }

    public enum Outcome: Sendable {
        case stopped
        case failed(Error)
        case timedOut
    }

    public typealias Stop = (@escaping @Sendable (Error?) -> Void) -> Void
    public typealias TimeoutScheduler = (TimeInterval, @escaping () -> Void) -> Void

    private enum State { case idle, stopping, finished }
    private let lock = NSLock()
    private var state = State.idle
    private var reason: Reason = .signal
    private var readyToStop: Bool
    private var stopStarted = false
    private let stop: Stop
    private let notifyParent: () -> Void
    private let completion: (Reason, Outcome) -> Void
    private let timeout: TimeInterval
    private let scheduleTimeout: TimeoutScheduler

    public init(
        readyToStop: Bool = true,
        timeout: TimeInterval = 10,
        scheduleTimeout: @escaping TimeoutScheduler = { delay, action in
            let work = DispatchWorkItem(block: action)
            DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + delay, execute: work)
        },
        notifyParent: @escaping () -> Void,
        stop: @escaping Stop,
        completion: @escaping (Reason, Outcome) -> Void
    ) {
        self.readyToStop = readyToStop
        self.timeout = timeout
        self.scheduleTimeout = scheduleTimeout
        self.notifyParent = notifyParent
        self.stop = stop
        self.completion = completion
    }

    public func request(_ reason: Reason) {
        // This must precede both stop and the single-flight guard: a signal or
        // watchdog shutdown must never suppress a later sleep notification.
        if case .sleep = reason { notifyParent() }

        lock.lock()
        guard state == .idle else {
            lock.unlock()
            return
        }
        state = .stopping
        self.reason = reason
        lock.unlock()

        // The deadline includes any pending startup. No semaphore or queue wait.
        // Late timeout/stop callbacks are harmless; only the first can complete.
        scheduleTimeout(timeout) { [weak self] in self?.finish(.timedOut) }
        stopIfReady()
    }

    /// Defer stopCapture until startCapture has returned, to avoid overlapping
    /// start/stop calls. A shutdown requested during startup is still timed out.
    public func allowStop() {
        lock.lock()
        readyToStop = true
        lock.unlock()
        stopIfReady()
    }

    private func stopIfReady() {
        lock.lock()
        guard state == .stopping, readyToStop, !stopStarted else {
            lock.unlock()
            return
        }
        stopStarted = true
        lock.unlock()

        stop { [weak self] error in
            if let error {
                self?.finish(.failed(error))
            } else {
                self?.finish(.stopped)
            }
        }
    }

    private func finish(_ outcome: Outcome) {
        lock.lock()
        guard state == .stopping else {
            lock.unlock()
            return
        }
        state = .finished
        let reason = self.reason
        lock.unlock()
        completion(reason, outcome)
    }
}
