import AppKit
import Foundation

let size = 512
guard CommandLine.arguments.count == 2 else {
    fputs("usage: swift scripts/generate-icon.swift <output.png>\n", stderr)
    exit(2)
}

guard let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: size,
    pixelsHigh: size,
    bitsPerSample: 8,
    samplesPerPixel: 4,
    hasAlpha: true,
    isPlanar: false,
    colorSpaceName: .deviceRGB,
    bytesPerRow: 0,
    bitsPerPixel: 0
) else {
    fputs("could not allocate icon bitmap\n", stderr)
    exit(1)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
NSGraphicsContext.current?.imageInterpolation = .high

NSColor(calibratedRed: 23 / 255, green: 27 / 255, blue: 34 / 255, alpha: 1).setFill()
NSBezierPath(roundedRect: NSRect(x: 0, y: 0, width: size, height: size), xRadius: 112, yRadius: 112).fill()

func ring(radius: CGFloat, width: CGFloat, color: NSColor) {
    color.setStroke()
    let path = NSBezierPath(ovalIn: NSRect(x: 256 - radius, y: 256 - radius, width: radius * 2, height: radius * 2))
    path.lineWidth = width
    path.stroke()
}

ring(radius: 158, width: 24, color: NSColor(calibratedWhite: 0.95, alpha: 1))
ring(radius: 103, width: 22, color: NSColor(calibratedRed: 98 / 255, green: 184 / 255, blue: 170 / 255, alpha: 1))

let aperture = NSBezierPath()
aperture.move(to: NSPoint(x: 256, y: 359))
aperture.line(to: NSPoint(x: 345, y: 307))
aperture.line(to: NSPoint(x: 256, y: 153))
aperture.line(to: NSPoint(x: 167, y: 307))
aperture.close()
NSColor(calibratedRed: 98 / 255, green: 184 / 255, blue: 170 / 255, alpha: 1).setFill()
aperture.fill()

NSColor(calibratedRed: 23 / 255, green: 27 / 255, blue: 34 / 255, alpha: 1).setFill()
NSBezierPath(ovalIn: NSRect(x: 221, y: 221, width: 70, height: 70)).fill()
NSGraphicsContext.restoreGraphicsState()

guard let data = bitmap.representation(using: .png, properties: [:]) else {
    fputs("could not encode icon PNG\n", stderr)
    exit(1)
}
try data.write(to: URL(fileURLWithPath: CommandLine.arguments[1]), options: .atomic)
