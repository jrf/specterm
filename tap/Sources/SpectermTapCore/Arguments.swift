public func sampleRate(from arguments: [String]) -> Double {
    guard let index = arguments.firstIndex(of: "--sample-rate"),
          index + 1 < arguments.count,
          let rate = Double(arguments[index + 1]) else {
        return 48000
    }
    return rate
}
