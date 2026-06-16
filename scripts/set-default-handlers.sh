#!/usr/bin/env bash
# Make Popdoc the DEFAULT handler for the Office / OpenDocument UTIs it claims.
#
# Why a script: macOS never auto-steals the default from Word/Pages/Numbers just
# because Popdoc *claims* a UTI (that only makes it a candidate in "Open with…").
# Setting the default is an explicit, per-UTI action via Launch Services
# (LSSetDefaultRoleHandlerForContentType). `duti` wraps the same call but isn't
# installed here, so we drive it with a tiny inline Swift program.
#
# Usage:
#   scripts/set-default-handlers.sh           # Office types only (docx/xls/ppt families)
#   scripts/set-default-handlers.sh --all     # + OpenDocument + rtf/csv/tsv
#
# The bundle id must already be registered with Launch Services
# (lsregister -f /Applications/Popdoc.app).

set -euo pipefail

BUNDLE_ID="io.popdoc.app"

OFFICE_UTIS=(
  org.openxmlformats.wordprocessingml.document
  com.microsoft.word.doc
  org.openxmlformats.spreadsheetml.sheet
  com.microsoft.excel.xls
  org.openxmlformats.presentationml.presentation
  com.microsoft.powerpoint.ppt
)
EXTRA_UTIS=(
  org.oasis-open.opendocument.text
  org.oasis-open.opendocument.spreadsheet
  org.oasis-open.opendocument.presentation
  public.rtf
  public.comma-separated-values-text
  public.tab-separated-values-text
)

UTIS=("${OFFICE_UTIS[@]}")
if [[ "${1:-}" == "--all" ]]; then
  UTIS+=("${EXTRA_UTIS[@]}")
fi

echo "==> Setting $BUNDLE_ID as default handler for ${#UTIS[@]} UTIs"
for uti in "${UTIS[@]}"; do
  /usr/bin/swift - "$BUNDLE_ID" "$uti" <<'SWIFT'
import CoreServices
import Foundation
let bundleId = CommandLine.arguments[1] as CFString
let uti = CommandLine.arguments[2] as CFString
let status = LSSetDefaultRoleHandlerForContentType(uti, .all, bundleId)
if status == noErr {
  print("   ok   \(CommandLine.arguments[2])")
} else {
  FileHandle.standardError.write("   FAIL \(CommandLine.arguments[2]) (OSStatus \(status))\n".data(using: .utf8)!)
}
SWIFT
done
echo "==> Done. Verify in Finder: right-click a .docx → Get Info → Open with."
