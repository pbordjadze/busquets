// Every toy's device shell (toys/README.md): a UIKit app whose one view is a CAMetalLayer the toy's
// Rust library draws on (toy-kit's C ABI, toy.h). Touches go in as pointer events in points; a
// display link asks for a frame at the screen's rate; the toy's sound, if it has one, is pulled by an
// AVAudioEngine source node on the audio thread. Nothing else lives here: the library is the toy.
import UIKit
import QuartzCore
import AVFoundation

final class ToyView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }
    var shell: OpaquePointer?
    var synth: OpaquePointer?
    var link: CADisplayLink?
    var engine: AVAudioEngine?
    var touchIds: [ObjectIdentifier: UInt64] = [:]
    var nextTouchId: UInt64 = 1
    let haptic = UIImpactFeedbackGenerator(style: .light)

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = .black
        (layer as? CAMetalLayer)?.framebufferOnly = true
    }
    required init?(coder: NSCoder) { fatalError() }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        guard window != nil else { link?.invalidate(); link = nil; return }
        let scale = window?.screen.nativeScale ?? UIScreen.main.scale
        contentScaleFactor = scale
        (layer as? CAMetalLayer)?.contentsScale = scale
        start()
    }

    func start() {
        guard shell == nil, let metal = layer as? CAMetalLayer else { return }
        let w = UInt32(max(bounds.width * metal.contentsScale, 1)), h = UInt32(max(bounds.height * metal.contentsScale, 1))
        metal.drawableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
        let assets = Bundle.main.resourcePath! + "/data"
        shell = assets.withCString { toy_new(Unmanaged.passUnretained(metal).toOpaque(), w, h, Double(metal.contentsScale), $0) }
        guard let shell else { NSLog("toy: the library did not start"); return }
        toy_set_reduce_motion(shell, UIAccessibility.isReduceMotionEnabled)
        toy_set_night(shell, traitCollection.userInterfaceStyle == .dark)
        let store = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("toy.json").path
        store.withCString { toy_store(shell, $0) }
        startSound()
        let center = NotificationCenter.default
        center.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_pause(shell) }
            self?.engine?.pause()
        }
        center.addObserver(forName: UIApplication.willEnterForegroundNotification, object: nil, queue: .main) { [weak self] _ in
            try? self?.engine?.start()
        }
        center.addObserver(forName: UIApplication.willTerminateNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_pause(shell) }
        }
        center.addObserver(forName: UIAccessibility.reduceMotionStatusDidChangeNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_set_reduce_motion(shell, UIAccessibility.isReduceMotionEnabled) }
        }
        let l = CADisplayLink(target: self, selector: #selector(tick(_:)))
        l.preferredFrameRateRange = CAFrameRateRange(minimum: 30, maximum: 120, preferred: 120)
        l.add(to: .main, forMode: .common)
        link = l
        becomeFirstResponder()
    }

    /// The toy's sound: an engine whose source node asks the library for samples (the ambient
    /// category: the silent switch silences it, and other apps' audio plays on). The library fills
    /// interleaved stereo into a scratch buffer allocated here, never on the audio thread; it is then
    /// copied into whatever layout the engine hands over (two deinterleaved buffers on a device, or
    /// one interleaved), in chunks no larger than the scratch.
    static let scratchFrames = 4096
    let scratch = UnsafeMutablePointer<Float>.allocate(capacity: ToyView.scratchFrames * 2)
    func startSound() {
        guard let shell, let s = toy_synth(shell) else { return }
        synth = s
        try? AVAudioSession.sharedInstance().setCategory(.ambient, mode: .default, options: [.mixWithOthers])
        try? AVAudioSession.sharedInstance().setActive(true)
        let engine = AVAudioEngine()
        let hw = engine.outputNode.outputFormat(forBus: 0).sampleRate
        let rate = hw > 0 ? hw : 48000
        guard let format = AVAudioFormat(standardFormatWithSampleRate: rate, channels: 2) else { return }
        let ptr = OpaquePointer(UnsafeMutableRawPointer(s))
        let scratch = self.scratch
        let cap = ToyView.scratchFrames
        let node = AVAudioSourceNode(format: format) { silence, _, frames, list -> OSStatus in
            let buffers = UnsafeMutableAudioBufferListPointer(list)
            let total = Int(frames)
            var done = 0
            while done < total {
                let n = min(cap, total - done)
                toy_audio(ptr, scratch, UInt32(n), 2, rate)
                if buffers.count >= 2 {
                    // deinterleaved: one buffer a channel
                    for c in 0..<2 {
                        guard let d = buffers[c].mData?.assumingMemoryBound(to: Float.self) else { continue }
                        let room = Int(buffers[c].mDataByteSize) / MemoryLayout<Float>.size
                        for i in 0..<n where done + i < room { d[done + i] = scratch[2 * i + c] }
                    }
                } else if buffers.count == 1, let d = buffers[0].mData?.assumingMemoryBound(to: Float.self) {
                    let ch = max(Int(buffers[0].mNumberChannels), 1)
                    let room = Int(buffers[0].mDataByteSize) / MemoryLayout<Float>.size
                    for i in 0..<n {
                        for c in 0..<ch where (done + i) * ch + c < room { d[(done + i) * ch + c] = scratch[2 * i + min(c, 1)] }
                    }
                }
                done += n
            }
            silence.pointee = false
            return noErr
        }
        engine.attach(node)
        engine.connect(node, to: engine.mainMixerNode, format: format)
        do { try engine.start(); self.engine = engine } catch { NSLog("toy: no sound: \(error)") }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        guard let metal = layer as? CAMetalLayer, let shell else { return }
        let w = UInt32(max(bounds.width * metal.contentsScale, 1)), h = UInt32(max(bounds.height * metal.contentsScale, 1))
        if metal.drawableSize != CGSize(width: CGFloat(w), height: CGFloat(h)) {
            metal.drawableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
            toy_resize(shell, w, h, Double(metal.contentsScale))
        }
    }

    @objc func tick(_ link: CADisplayLink) {
        guard let shell else { return }
        toy_frame(shell, link.targetTimestamp)
        if toy_take_haptic(shell) { haptic.impactOccurred() }
    }

    override func traitCollectionDidChange(_ previous: UITraitCollection?) {
        super.traitCollectionDidChange(previous)
        if let shell { toy_set_night(shell, traitCollection.userInterfaceStyle == .dark) }
    }
    override var canBecomeFirstResponder: Bool { true }
    override func motionEnded(_ motion: UIEvent.EventSubtype, with event: UIEvent?) {
        if motion == .motionShake, let shell { toy_shaken(shell) }
    }

    func send(_ touches: Set<UITouch>, _ phase: UInt32) {
        guard let shell else { return }
        for t in touches {
            let key = ObjectIdentifier(t)
            let id: UInt64
            if let known = touchIds[key] { id = known } else { id = nextTouchId; nextTouchId += 1; touchIds[key] = id }
            let p = t.location(in: self)
            toy_pointer(shell, id, phase, Double(p.x), Double(p.y), t.timestamp)
            if phase >= 2 { touchIds.removeValue(forKey: key) }
        }
    }
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 0) }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 1) }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 2) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 3) }
}

final class ToyController: UIViewController {
    override func loadView() { view = ToyView(frame: UIScreen.main.bounds) }
    override var prefersStatusBarHidden: Bool { true }
    override var prefersHomeIndicatorAutoHidden: Bool { true }
    override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { .all }
}

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        let w = UIWindow(frame: UIScreen.main.bounds)
        w.rootViewController = ToyController()
        w.makeKeyAndVisible()
        window = w
        return true
    }
}
