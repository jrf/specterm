import XCTest
import SpectermTapCore

final class ShutdownCoordinatorTests: XCTestCase {
    private enum TestError: Error { case stop }

    private final class Harness {
        var events: [String] = []
        var deadlines: [TimeInterval] = []
        var fireTimeout: (() -> Void)?
        var finishStop: ((Error?) -> Void)?
        var results: [(ShutdownCoordinator.Reason, ShutdownCoordinator.Outcome)] = []

        func coordinator(ready: Bool = true) -> ShutdownCoordinator {
            ShutdownCoordinator(
                readyToStop: ready,
                scheduleTimeout: { delay, action in
                    self.deadlines.append(delay)
                    self.fireTimeout = action
                },
                notifyParent: { self.events.append("notify") },
                stop: { done in
                    self.events.append("stop")
                    self.finishStop = done
                },
                completion: { reason, outcome in
                    self.events.append("complete")
                    self.results.append((reason, outcome))
                }
            )
        }
    }

    func testSleepNotifiesBeforeStopAndReturnsWithoutWaiting() {
        let h = Harness()
        let coordinator = h.coordinator()
        coordinator.request(.sleep)
        XCTAssertEqual(h.events, ["notify", "stop"])
        XCTAssertEqual(h.deadlines, [10])
        XCTAssertTrue(h.results.isEmpty)
        h.finishStop?(nil)
        XCTAssertEqual(h.events, ["notify", "stop", "complete"])
        XCTAssertEqual(h.results.first?.0, .sleep)
        guard case .stopped? = h.results.first?.1 else {
            return XCTFail("Expected successful shutdown")
        }
        h.fireTimeout?()
        h.finishStop?(TestError.stop)
        XCTAssertEqual(h.results.count, 1)
    }

    func testSleepAlwaysNotifiesDuringAndAfterAnotherShutdown() {
        for first: ShutdownCoordinator.Reason in [.signal, .parentExited, .captureError] {
            let h = Harness()
            let coordinator = h.coordinator()
            coordinator.request(first)
            coordinator.request(.signal)
            coordinator.request(.parentExited)
            coordinator.request(.sleep)
            coordinator.request(.sleep)
            XCTAssertEqual(h.events, ["stop", "notify", "notify"])
            XCTAssertEqual(h.deadlines, [10])
            XCTAssertTrue(h.results.isEmpty)
            h.finishStop?(nil)
            coordinator.request(.sleep)
            XCTAssertEqual(h.events, ["stop", "notify", "notify", "complete", "notify"])
            XCTAssertEqual(h.results.count, 1)
            XCTAssertEqual(h.results.first?.0, first)
        }
    }

    func testStopErrorCompletesOnce() {
        let h = Harness()
        let coordinator = h.coordinator()
        coordinator.request(.signal)
        h.finishStop?(TestError.stop)
        h.fireTimeout?()
        h.finishStop?(nil)
        XCTAssertEqual(h.results.count, 1)
        guard case .failed(let error)? = h.results.first?.1 else {
            return XCTFail("Expected stop error")
        }
        XCTAssertTrue(error is TestError)
    }

    func testTimeoutWinsAndLateStopCannotExitAgain() {
        let h = Harness()
        let coordinator = h.coordinator()
        coordinator.request(.parentExited)
        h.fireTimeout?()
        h.finishStop?(nil)
        h.fireTimeout?()
        coordinator.request(.signal)
        XCTAssertEqual(h.events, ["stop", "complete"])
        XCTAssertEqual(h.results.count, 1)
        guard case .timedOut? = h.results.first?.1 else {
            return XCTFail("Expected timeout")
        }
    }

    func testShutdownDuringStartupDefersStopButNotNotificationOrDeadline() {
        let h = Harness()
        let coordinator = h.coordinator(ready: false)
        coordinator.request(.sleep)
        XCTAssertEqual(h.events, ["notify"])
        XCTAssertEqual(h.deadlines, [10])
        coordinator.allowStop()
        coordinator.allowStop()
        coordinator.request(.signal)
        XCTAssertEqual(h.events, ["notify", "stop"])
        h.finishStop?(nil)
        XCTAssertEqual(h.results.count, 1)
    }

    func testStartupTimeoutPreventsLateStop() {
        let h = Harness()
        let coordinator = h.coordinator(ready: false)
        coordinator.request(.signal)
        h.fireTimeout?()
        coordinator.allowStop()
        XCTAssertEqual(h.events, ["complete"])
        XCTAssertEqual(h.results.count, 1)
        guard case .timedOut? = h.results.first?.1 else {
            return XCTFail("Expected startup timeout")
        }
    }

    func testSynchronousStopCompletionIsSafe() {
        var completions = 0
        var fireTimeout: (() -> Void)?
        let coordinator = ShutdownCoordinator(
            scheduleTimeout: { _, action in fireTimeout = action },
            notifyParent: {},
            stop: { done in done(nil); done(TestError.stop) },
            completion: { _, _ in completions += 1 }
        )
        coordinator.request(.signal)
        fireTimeout?()
        XCTAssertEqual(completions, 1)
    }

    func testConcurrentRequestsAndCompetingCallbacksAreSingleFlight() {
        let lock = NSLock()
        var stops = 0
        var completions = 0
        var finishStop: ((Error?) -> Void)?
        var fireTimeout: (() -> Void)?
        let coordinator = ShutdownCoordinator(
            scheduleTimeout: { _, action in
                lock.lock()
                fireTimeout = action
                lock.unlock()
            },
            notifyParent: {},
            stop: { done in
                lock.lock()
                stops += 1
                finishStop = done
                lock.unlock()
            },
            completion: { _, _ in
                lock.lock()
                completions += 1
                lock.unlock()
            }
        )
        DispatchQueue.concurrentPerform(iterations: 100) { index in
            coordinator.request(index.isMultiple(of: 2) ? .signal : .parentExited)
        }
        XCTAssertEqual(stops, 1)
        XCTAssertEqual(completions, 0)
        let stopped = finishStop!
        let timedOut = fireTimeout!
        DispatchQueue.concurrentPerform(iterations: 100) { index in
            if index.isMultiple(of: 2) { stopped(nil) } else { timedOut() }
        }
        XCTAssertEqual(completions, 1)
    }
}
