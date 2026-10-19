$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$nativeBrandInputs = @(
  @{ Source = 'src-tauri/target/release/ydlite.exe'; Label = 'app' },
  @{ Source = 'src-tauri/target/release/bundle/nsis/YDLite_0.2.0_x64-setup.exe'; Label = 'setup' }
)
foreach ($nativeBrandInput in $nativeBrandInputs) {
  $nativeBrandSource = (Resolve-Path -LiteralPath $nativeBrandInput.Source).Path
  $nativeBrandHash = (Get-FileHash -LiteralPath $nativeBrandSource -Algorithm SHA256).Hash
  # A unique file path prevents Windows' shell icon cache from hiding a resource change.
  $nativeBrandCopy = Join-Path (Resolve-Path -LiteralPath 'artifacts/logo').Path ($nativeBrandInput.Label + '-' + $nativeBrandHash.Substring(0, 16) + '.exe')
  if (Test-Path -LiteralPath $nativeBrandCopy) {
    if ((Get-FileHash -LiteralPath $nativeBrandCopy).Hash -ne $nativeBrandHash) { throw 'Existing inspection copy changed' }
  } else {
    Copy-Item -LiteralPath $nativeBrandSource -Destination $nativeBrandCopy
  }
  $nativeBrandIcon = [System.Drawing.Icon]::ExtractAssociatedIcon($nativeBrandCopy)
  if ($null -eq $nativeBrandIcon) { throw 'No embedded executable icon' }
  $nativeBrandBitmap = $nativeBrandIcon.ToBitmap()
  try {
    $nativeBrandBluePixels = 0
    for ($nativeBrandY = 0; $nativeBrandY -lt $nativeBrandBitmap.Height; $nativeBrandY++) {
      for ($nativeBrandX = 0; $nativeBrandX -lt $nativeBrandBitmap.Width; $nativeBrandX++) {
        $nativeBrandPixel = $nativeBrandBitmap.GetPixel($nativeBrandX, $nativeBrandY)
        if ($nativeBrandPixel.R -eq 37 -and $nativeBrandPixel.G -eq 90 -and $nativeBrandPixel.B -eq 214 -and $nativeBrandPixel.A -eq 255) { $nativeBrandBluePixels++ }
      }
    }
    if ($nativeBrandBluePixels -lt ($nativeBrandBitmap.Width * $nativeBrandBitmap.Height / 3)) { throw ('Old or unexpected embedded icon: ' + $nativeBrandInput.Label) }
    $nativeBrandImage = Join-Path (Resolve-Path -LiteralPath 'artifacts/logo').Path ($nativeBrandInput.Label + '-native-icon.png')
    $nativeBrandBitmap.Save($nativeBrandImage, [System.Drawing.Imaging.ImageFormat]::Png)
    'PASS: ' + $nativeBrandInput.Label + ' contains the new embedded blue logo'
  } finally {
    $nativeBrandBitmap.Dispose()
    $nativeBrandIcon.Dispose()
  }
}
