import AppKit
import Foundation

let directory = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)

for size in [16, 32, 128, 256, 512] {
  for scale in [1, 2] {
    let pixels = size * scale
    let bitmap = NSBitmapImageRep(
      bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
      bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
      colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
    let transform = NSAffineTransform()
    transform.scale(by: CGFloat(pixels) / 1024)
    transform.concat()

    let background = NSBezierPath(
      roundedRect: NSRect(x: 64, y: 64, width: 896, height: 896), xRadius: 205, yRadius: 205)
    NSGradient(
      starting: NSColor(red: 0.12, green: 0.23, blue: 0.30, alpha: 1),
      ending: NSColor(red: 0.035, green: 0.08, blue: 0.13, alpha: 1))!.draw(
        in: background, angle: -90)

    let frame = NSBezierPath(
      roundedRect: NSRect(x: 196, y: 250, width: 632, height: 524), xRadius: 72, yRadius: 72)
    NSColor.white.withAlphaComponent(0.92).setStroke()
    frame.lineWidth = 28
    frame.stroke()
    let divider = NSBezierPath()
    divider.move(to: NSPoint(x: 210, y: 650))
    divider.line(to: NSPoint(x: 814, y: 650))
    divider.lineWidth = 22
    divider.stroke()
    NSColor.white.withAlphaComponent(0.7).setFill()
    for x in [265, 325, 385] {
      NSBezierPath(ovalIn: NSRect(x: x, y: 699, width: 22, height: 22)).fill()
    }

    let prompt = NSBezierPath()
    prompt.move(to: NSPoint(x: 300, y: 552))
    prompt.line(to: NSPoint(x: 405, y: 462))
    prompt.line(to: NSPoint(x: 300, y: 372))
    prompt.lineWidth = 48
    prompt.lineCapStyle = .round
    prompt.lineJoinStyle = .round
    NSColor(red: 0.34, green: 0.91, blue: 0.69, alpha: 1).setStroke()
    prompt.stroke()
    let cursor = NSBezierPath()
    cursor.move(to: NSPoint(x: 495, y: 372))
    cursor.line(to: NSPoint(x: 687, y: 372))
    cursor.lineWidth = 48
    cursor.lineCapStyle = .round
    cursor.stroke()
    NSGraphicsContext.restoreGraphicsState()
    let suffix = scale == 2 ? "@2x" : ""
    let output = directory.appendingPathComponent("icon_\(size)x\(size)\(suffix).png")
    try bitmap.representation(using: .png, properties: [:])!.write(to: output)
  }
}
