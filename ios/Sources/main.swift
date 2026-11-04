// Every take's device shell (from terra-incognita's toys/ios, NOTICE.md): a UIKit app whose one view is a CAMetalLayer the take's
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
    /// The info mark's one line, in the system's text (it follows the reader's text size; VoiceOver
    /// reads it), shown while the kit plays the mark's demo (toy-kit's info.rs).
    let caption = UILabel()
    var captionText: String?
    lazy var infoElement = InfoElement(accessibilityContainer: self)
    lazy var rulesElement = RulesElement(accessibilityContainer: self)
    /// The rules (the rules mark's; toy-kit's info.rs › Rules) in the system's text: the toy's few
    /// rules, then one dimmer line of what is measured and what is drawn, on a dark glass over the live
    /// scene, above the marks. A tap anywhere closes it; it scrolls if the reader's text size needs.
    /// tools/kit/shell_text.py draws the same for the Linux check's screenshots: change both together.
    let rules = UIVisualEffectView(effect: UIBlurEffect(style: .systemUltraThinMaterialDark))
    let rulesScroll = UIScrollView()
    let rulesLabel = UILabel()
    var rulesText: String?
    /// (TOY_OPEN=rules or info, set by tools/toys/sim_launch.sh: opens it once, TOY_OPEN_AFTER seconds in)
    var openAt: (String, Double)?

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = .black
        (layer as? CAMetalLayer)?.framebufferOnly = true
        caption.font = UIFont.preferredFont(forTextStyle: .callout)
        caption.adjustsFontForContentSizeCategory = true
        caption.adjustsFontSizeToFitWidth = true
        caption.minimumScaleFactor = 0.7
        caption.numberOfLines = 1
        caption.textAlignment = .center
        caption.textColor = UIColor(white: 1.0, alpha: 0.92)
        caption.layer.shadowColor = UIColor.black.cgColor
        caption.layer.shadowOpacity = 0.9
        caption.layer.shadowRadius = 6
        caption.layer.shadowOffset = .zero
        caption.alpha = 0
        caption.isUserInteractionEnabled = false
        addSubview(caption)
        rules.layer.cornerRadius = 16
        rules.layer.cornerCurve = .continuous
        rules.clipsToBounds = true
        // (a dark tint on the glass, so it stays dark over paper as over night)
        rules.contentView.backgroundColor = UIColor(white: 0.06, alpha: 0.62)
        rules.alpha = 0
        rules.isHidden = true
        rulesLabel.numberOfLines = 0
        rulesLabel.isAccessibilityElement = true
        rulesScroll.addSubview(rulesLabel)
        rules.contentView.addSubview(rulesScroll)
        rules.addGestureRecognizer(UITapGestureRecognizer(target: self, action: #selector(closeRules)))
        addSubview(rules)
        let env = ProcessInfo.processInfo.environment
        if let what = env["TOY_OPEN"] { openAt = (what, CACurrentMediaTime() + (Double(env["TOY_OPEN_AFTER"] ?? "") ?? 8)) }
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
        // the toy's log (a panic's message, a GPU error, a toy that could not start), in the app's
        // Documents, which the Files app shows: On My iPhone › <the toy> › toy-log.txt
        let log = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("toy-log.txt").path
        log.withCString { toy_set_log($0) }
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
        if rulesText != nil { layoutRules() }
    }

    @objc func tick(_ link: CADisplayLink) {
        guard let shell else { return }
        toy_frame(shell, link.targetTimestamp)
        if toy_take_haptic(shell) { haptic.impactOccurred() }
        if let o = openAt, CACurrentMediaTime() >= o.1 {
            openAt = nil
            if o.0 == "rules" { toy_rules_open(shell) } else { toy_info_open(shell, CACurrentMediaTime()) }
        }
        showCaption(toy_info_line(shell).map { String(cString: $0) })
        showRules(toy_rules_text(shell).map { String(cString: $0) })
    }

    @objc func closeRules() {
        if let shell, rulesText != nil { toy_rules_open(shell) }
    }

    func showRules(_ text: String?) {
        guard text != rulesText else { return }
        rulesText = text
        guard let text else {
            UIView.animate(withDuration: 0.3, animations: { self.rules.alpha = 0 }) { _ in if self.rulesText == nil { self.rules.isHidden = true } }
            UIAccessibility.post(notification: .layoutChanged, argument: nil)
            return
        }
        let parts = text.components(separatedBy: "\n\n")
        let body = UIFont.preferredFont(forTextStyle: .callout), note = UIFont.preferredFont(forTextStyle: .footnote)
        let lines = NSMutableParagraphStyle()
        lines.paragraphSpacing = 7
        let noteStyle = NSMutableParagraphStyle()
        noteStyle.paragraphSpacingBefore = 6
        let s = NSMutableAttributedString(string: parts[0], attributes: [.font: body, .foregroundColor: UIColor(white: 1, alpha: 0.95), .paragraphStyle: lines])
        if parts.count > 1 {
            s.append(NSAttributedString(string: "\n" + parts[1...].joined(separator: "\n"), attributes: [.font: note, .foregroundColor: UIColor(white: 1, alpha: 0.64), .paragraphStyle: noteStyle]))
        }
        rulesLabel.attributedText = s
        rulesLabel.accessibilityLabel = parts.joined(separator: ". ")
        layoutRules()
        rules.isHidden = false
        UIView.animate(withDuration: 0.25) { self.rules.alpha = 1 }
        UIAccessibility.post(notification: .screenChanged, argument: rulesLabel)
    }

    /// The panel's place: at most 480 points wide, centred, its foot 60 points above the screen's
    /// (clear of the marks), as tall as its text up to the safe area's top.
    func layoutRules() {
        let w = min(bounds.width - 32, 480), pad = UIEdgeInsets(top: 14, left: 16, bottom: 14, right: 16)
        let text = rulesLabel.sizeThatFits(CGSize(width: w - pad.left - pad.right, height: .greatestFiniteMagnitude))
        let foot = bounds.height - 60
        let h = min(text.height + pad.top + pad.bottom, foot - safeAreaInsets.top - 8)
        rules.frame = CGRect(x: (bounds.width - w) / 2, y: foot - h, width: w, height: h)
        rulesScroll.frame = rules.contentView.bounds.inset(by: pad)
        rulesLabel.frame = CGRect(origin: .zero, size: CGSize(width: w - pad.left - pad.right, height: text.height))
        rulesScroll.contentSize = rulesLabel.frame.size
    }

    func showCaption(_ text: String?) {
        guard text != captionText else { return }
        captionText = text
        if let text {
            caption.text = text
            let w = min(bounds.width - 48, 560)
            caption.frame = CGRect(x: (bounds.width - w) / 2, y: bounds.height - safeAreaInsets.bottom - 96, width: w, height: 44)
            UIView.animate(withDuration: 0.25) { self.caption.alpha = 1 }
            UIAccessibility.post(notification: .announcement, argument: text)
        } else {
            UIView.animate(withDuration: 0.35) { self.caption.alpha = 0 }
        }
    }

    /// The marks, for VoiceOver: "About this app", activated to open its demo; "How it works", the
    /// rules; and the rules' text while it shows.
    override var accessibilityElements: [Any]? {
        get {
            var x = 0.0, y = 0.0
            var out: [Any] = []
            guard let shell else { return [] }
            if rulesText != nil { out.append(rulesLabel) }
            if toy_rules_spot(shell, &x, &y) {
                rulesElement.accessibilityFrameInContainerSpace = CGRect(x: x - 22, y: y - 22, width: 44, height: 44)
                out.append(rulesElement)
            }
            if toy_info_spot(shell, &x, &y) {
                infoElement.accessibilityFrameInContainerSpace = CGRect(x: x - 22, y: y - 22, width: 44, height: 44)
                out.append(infoElement)
            }
            return out
        }
        set {}
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

final class InfoElement: UIAccessibilityElement {
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityLabel = "About this app"
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView, let shell = view.shell else { return false }
        toy_info_open(shell, CACurrentMediaTime())
        return true
    }
}

final class RulesElement: UIAccessibilityElement {
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityLabel = "How it works"
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView, let shell = view.shell else { return false }
        toy_rules_open(shell)
        return true
    }
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

/// A fetch for the library (crates/real: StatsBomb's open data, fetched at run time, never bundled):
/// `done(token, bytes, count, ok)` is called on URLSession's thread when it has come.
@_cdecl("toy_fetch")
public func toyFetch(_ url: UnsafePointer<CChar>, _ token: UInt64, _ done: @convention(c) (UInt64, UnsafePointer<UInt8>?, Int, Bool) -> Void) {
    guard let u = URL(string: String(cString: url)) else { done(token, nil, 0, false); return }
    URLSession.shared.dataTask(with: u) { data, response, _ in
        let ok = ((response as? HTTPURLResponse)?.statusCode ?? 0) == 200
        if ok, let data {
            data.withUnsafeBytes { raw in done(token, raw.bindMemory(to: UInt8.self).baseAddress, data.count, true) }
        } else {
            done(token, nil, 0, false)
        }
    }.resume()
}

