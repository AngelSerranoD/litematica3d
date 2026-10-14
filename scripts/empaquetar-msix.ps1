# Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
# Ver LICENSE y NOTICE.

<#
.SYNOPSIS
  Compila Schematic Lab y genera el paquete MSIX para la Microsoft Store.

.DESCRIPTION
  1. `npx tauri build --no-bundle` (frontend + exe en modo release).
  2. Monta el diseño del paquete en src-tauri\target\msix\diseno: el exe, los
     avisos legales, los logos de src-tauri\msix\Assets y el AppxManifest.xml
     sacado de la plantilla src-tauri\msix\AppxManifest.xml.
  3. makepri (índice de recursos, para que Windows elija el logo de cada tamaño)
     y makeappx (empaqueta) del Windows SDK.
  Resultado: src-tauri\target\msix\SchematicLab_<versión>_x64.msix, sin firmar
  (la Store firma el paquete al publicarlo). Todo queda dentro de
  src-tauri\target, que no va a git.

  Vía elegida, lo descartado y cómo probarlo: docs\distribucion.md.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\empaquetar-msix.ps1

.EXAMPLE
  # Con la identidad que da Partner Center (Producto > Identidad del producto):
  powershell -ExecutionPolicy Bypass -File scripts\empaquetar-msix.ps1 `
    -IdentityName '12345AngelSerrano.SchematicLab' `
    -Publisher 'CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX'
#>
[CmdletBinding()]
param(
    # ---- Identidad del paquete -------------------------------------------
    # Definitivos: los de Partner Center («Identidad del producto»), copiados
    # al reservar «Schematic Lab» (2026-10-14). Partner Center quita los
    # acentos del nombre, por eso «ngelSerranoDomnguez».
    #   Package/Identity/Name                -> IdentityName
    #   Package/Identity/Publisher           -> Publisher
    #   Package/Properties/PublisherDisplayName -> PublisherDisplayName
    [string]$IdentityName = 'ngelSerranoDomnguez.SchematicLab',
    [string]$Publisher = 'CN=D55F1367-3A8A-40AF-8B0C-2A907C0785BE',
    [string]$PublisherDisplayName = 'Ángel Serrano Domínguez',
    # -----------------------------------------------------------------------

    # No compila: reutiliza el exe que ya haya en src-tauri\target\release.
    [switch]$SinCompilar
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$raiz = Split-Path -Parent $PSScriptRoot
$srcTauri = Join-Path $raiz 'src-tauri'
$release = Join-Path $srcTauri 'target\release'
$salida = Join-Path $srcTauri 'target\msix'
$diseno = Join-Path $salida 'diseno'

function Fallar([string]$mensaje) {
    Write-Host "ERROR: $mensaje" -ForegroundColor Red
    exit 1
}

# Herramienta del Windows SDK (la versión más reciente instalada, x64).
function Buscar-HerramientaSdk([string]$nombre) {
    $base = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $candidatas = Get-ChildItem -Path $base -Directory -Filter '10.*' -ErrorAction SilentlyContinue |
        Sort-Object { [version]$_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$nombre" } |
        Where-Object { Test-Path $_ }
    $ruta = @($candidatas) | Select-Object -First 1
    if (-not $ruta) {
        Fallar "No encuentro $nombre. Instala el Windows SDK (winget install Microsoft.WindowsSDK.10.0.26100)."
    }
    return $ruta
}

# Escapa un valor para meterlo en un atributo o texto XML.
function Escapar-Xml([string]$texto) {
    return [System.Security.SecurityElement]::Escape($texto)
}

# ---- Versión: la de package.json (fuente única), con cuarto número 0 --------
# La Store exige que el último número sea 0 (lo reserva para ella).
$paquete = Get-Content -Raw -Encoding UTF8 (Join-Path $raiz 'package.json') | ConvertFrom-Json
if ($paquete.version -notmatch '^(\d+)\.(\d+)\.(\d+)$') {
    Fallar "La versión de package.json ('$($paquete.version)') no es X.Y.Z."
}
$version = "$($paquete.version).0"

$makeappx = Buscar-HerramientaSdk 'makeappx.exe'
$makepri = Buscar-HerramientaSdk 'makepri.exe'

# ---- 1. Compilar --------------------------------------------------------------
if (-not $SinCompilar) {
    Write-Host '==> Compilando (npx tauri build --no-bundle)...'
    Push-Location $raiz
    try {
        & npx.cmd tauri build --no-bundle
        if ($LASTEXITCODE -ne 0) { Fallar 'tauri build ha fallado.' }
    }
    finally {
        Pop-Location
    }
}
$exe = Join-Path $release 'SchematicLab.exe'
if (-not (Test-Path $exe)) { Fallar "No existe $exe. Compila antes (sin -SinCompilar)." }

# ---- 2. Diseño del paquete ----------------------------------------------------
# La carpeta es salida generada por este script: se rehace entera cada vez.
# Si falla al vaciarla, suele ser porque la app de prueba está abierta.
if (Test-Path $diseno) {
    Remove-Item -Recurse -Force $diseno
}
New-Item -ItemType Directory -Force -Path $diseno | Out-Null

Copy-Item $exe $diseno
# Avisos legales: los mismos que lleva el instalador NSIS (bundle.resources de
# tauri.conf.json) más la licencia.
Copy-Item (Join-Path $raiz 'LICENSE') $diseno
Copy-Item (Join-Path $raiz 'NOTICE') $diseno
New-Item -ItemType Directory -Force -Path (Join-Path $diseno 'licencias') | Out-Null
Copy-Item (Join-Path $raiz 'src\assets\fuentes\OFL.txt') (Join-Path $diseno 'licencias\OFL.txt')
Copy-Item -Recurse (Join-Path $srcTauri 'msix\Assets') (Join-Path $diseno 'Assets')

$plantilla = [System.IO.File]::ReadAllText((Join-Path $srcTauri 'msix\AppxManifest.xml'))
$manifiesto = $plantilla.
    Replace('{{IDENTITY_NAME}}', (Escapar-Xml $IdentityName)).
    Replace('{{PUBLISHER}}', (Escapar-Xml $Publisher)).
    Replace('{{PUBLISHER_DISPLAY_NAME}}', (Escapar-Xml $PublisherDisplayName)).
    Replace('{{VERSION}}', $version)
if ($manifiesto -match '\{\{[A-Z_]+\}\}') { Fallar "Queda un marcador sin sustituir: $($Matches[0])" }
$rutaManifiesto = Join-Path $diseno 'AppxManifest.xml'
[System.IO.File]::WriteAllText($rutaManifiesto, $manifiesto, (New-Object System.Text.UTF8Encoding $false))

# ---- 3. Índice de recursos (resources.pri) -----------------------------------
# Sin él, Windows no sabe elegir entre Square44x44Logo.scale-200.png,
# .targetsize-24_altform-unplated.png, etc.
Write-Host '==> Generando resources.pri...'
$priconfig = Join-Path $salida 'priconfig.xml'
& $makepri createconfig /cf $priconfig /dq es-ES /pv 10.0.0 /o | Out-Null
if ($LASTEXITCODE -ne 0) { Fallar 'makepri createconfig ha fallado.' }
# La configuración por defecto reparte los recursos por escala e idioma en
# varios .pri (pensado para lotes .msixbundle). Con un solo .msix, los logos
# scale-200 quedarían fuera del resources.pri principal: se quita <packaging>.
$xmlPri = [xml](Get-Content -Raw -Encoding UTF8 $priconfig)
$empaquetado = $xmlPri.resources.SelectSingleNode('packaging')
if ($empaquetado) { [void]$xmlPri.resources.RemoveChild($empaquetado) }
$xmlPri.Save($priconfig)
& $makepri new /pr $diseno /cf $priconfig /mn $rutaManifiesto /of (Join-Path $diseno 'resources.pri') /o | Out-Null
if ($LASTEXITCODE -ne 0) { Fallar 'makepri new ha fallado.' }

# ---- 4. Empaquetar -------------------------------------------------------------
$msix = Join-Path $salida "SchematicLab_$($version)_x64.msix"
Write-Host '==> Empaquetando con makeappx...'
& $makeappx pack /d $diseno /p $msix /o | Out-Null
if ($LASTEXITCODE -ne 0) {
    # Repetir con la salida visible para ver el motivo.
    & $makeappx pack /d $diseno /p $msix /o
    Fallar 'makeappx ha fallado.'
}

$mib = [math]::Round((Get-Item $msix).Length / 1MB, 2)
Write-Host ''
Write-Host "Paquete: $msix ($mib MiB, versión $version, sin firmar)" -ForegroundColor Green
Write-Host "Identidad: $IdentityName / $Publisher"
Write-Host ''
Write-Host 'Probar sin firmar (modo desarrollador activo):'
Write-Host "  Add-AppxPackage -Register '$rutaManifiesto'"
Write-Host 'Quitarlo después:'
Write-Host "  Get-AppxPackage $IdentityName | Remove-AppxPackage"
