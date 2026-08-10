import XCTest
import SpectermTapCore

final class SpectermTapTests: XCTestCase {
    func testSampleRateDefaultsTo48000() {
        XCTAssertEqual(sampleRate(from: ["specterm-tap"]), 48000)
    }

    func testSampleRateReadsOptionValue() {
        XCTAssertEqual(
            sampleRate(from: ["specterm-tap", "--sample-rate", "44100"]),
            44100
        )
    }

    func testSampleRateDefaultsWhenValueIsMissingOrInvalid() {
        XCTAssertEqual(sampleRate(from: ["specterm-tap", "--sample-rate"]), 48000)
        XCTAssertEqual(
            sampleRate(from: ["specterm-tap", "--sample-rate", "invalid"]),
            48000
        )
    }
}
