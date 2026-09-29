// Print the CGWindowID of the largest on-screen cmux window (layer 0).
// Used by snap_cmux.sh: `screencapture -l <id>` needs a CGWindowID, which
// AppleScript cannot provide. Compiled on demand by the script.
import CoreGraphics
import Foundation

guard
    let info = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID)
        as? [[String: Any]]
else { exit(1) }

var best: (num: Int, area: Double)? = nil
for w in info {
    guard
        let owner = w[kCGWindowOwnerName as String] as? String,
        owner.lowercased().contains("cmux"),
        let layer = w[kCGWindowLayer as String] as? Int, layer == 0,
        let num = w[kCGWindowNumber as String] as? Int,
        let bounds = w[kCGWindowBounds as String] as? [String: Double]
    else { continue }
    let area = (bounds["Width"] ?? 0) * (bounds["Height"] ?? 0)
    if best == nil || area > best!.area { best = (num, area) }
}
guard let b = best else { exit(2) }
print(b.num)
