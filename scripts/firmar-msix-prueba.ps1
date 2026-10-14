# Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
# Ver LICENSE y NOTICE.

<#
.SYNOPSIS
  Firma el .msix con un certificado de prueba autofirmado y lo instala, SOLO
  para probarlo en este equipo. La Store no lo necesita: firma ella el paquete
  que se sube a Partner Center.

.DESCRIPTION
  Lo ejecuta Ángel a mano, en un PowerShell abierto como administrador, porque
  cambia almacenes de certificados del sistema:
    1. Crea (o reutiliza) un certificado de firma de código autofirmado en
       Cert:\CurrentUser\My con el mismo sujeto que el Publisher del paquete.
    2. Firma el .msix con signtool (SHA-256) usando ese certificado.
    3. Confía en él copiando la parte pública a Cert:\LocalMachine\TrustedPeople.
    4. Instala el paquete con Add-AppxPackage.
  Al final enseña cómo deshacerlo todo (no borra nada por su cuenta).

  Antes: generar el paquete con scripts\empaquetar-msix.ps1 y, si estaba
  registrado el diseño de desarrollo (Add-AppxPackage -Register), quitarlo:
    Get-AppxPackage AngelSerranoDominguez.Litematica3D | Remove-AppxPackage

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\firmar-msix-prueba.ps1
#>
[CmdletBinding()]
param(
    # Tiene que ser idéntico al Publisher con el que se generó el paquete.
    [string]$Publisher = 'CN=Angel Serrano Dominguez',
    # Por defecto, el .msix más reciente de src-tauri\target\msix.
    [string]$Msix
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$raiz = Split-Path -Parent $PSScriptRoot
$salida = Join-Path $raiz 'src-tauri\target\msix'

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).
    IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    Write-Host 'Abre PowerShell como administrador: hace falta para confiar en el certificado (LocalMachine\TrustedPeople).' -ForegroundColor Red
    exit 1
}

if (-not $Msix) {
    $Msix = Get-ChildItem $salida -Filter '*.msix' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $Msix -or -not (Test-Path $Msix)) {
    Write-Host 'No hay .msix. Genéralo antes con scripts\empaquetar-msix.ps1.' -ForegroundColor Red
    exit 1
}

$signtool = Get-ChildItem (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin') -Directory -Filter '10.*' |
    Sort-Object { [version]$_.Name } -Descending |
    ForEach-Object { Join-Path $_.FullName 'x64\signtool.exe' } |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $signtool) { Write-Host 'No encuentro signtool.exe (Windows SDK).' -ForegroundColor Red; exit 1 }

# 1. Certificado (se reutiliza si ya existe uno de prueba con ese sujeto).
$nombre = 'Litematica3D - prueba MSIX'
$cert = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { $_.Subject -eq $Publisher -and $_.FriendlyName -eq $nombre -and $_.NotAfter -gt (Get-Date) } |
    Select-Object -First 1
if (-not $cert) {
    Write-Host "==> Creando certificado de prueba '$Publisher'..."
    $cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher -FriendlyName $nombre `
        -KeyUsage DigitalSignature -CertStoreLocation Cert:\CurrentUser\My `
        -NotAfter (Get-Date).AddYears(1) `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
}

# 2. Firmar.
Write-Host '==> Firmando el paquete...'
& $signtool sign /fd SHA256 /sha1 $cert.Thumbprint /s My $Msix
if ($LASTEXITCODE -ne 0) { Write-Host 'signtool ha fallado.' -ForegroundColor Red; exit 1 }

# 3. Confiar en el certificado (solo la parte pública).
$cer = Join-Path $salida 'litematica3d-prueba.cer'
Export-Certificate -Cert $cert -FilePath $cer | Out-Null
Import-Certificate -FilePath $cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null

# 4. Instalar.
Write-Host '==> Instalando...'
Add-AppxPackage -Path $Msix

Write-Host ''
Write-Host 'Instalado. Ábrelo desde el menú Inicio (Litematica3D).' -ForegroundColor Green
Write-Host ''
Write-Host 'Para deshacerlo todo cuando acabes de probar:'
Write-Host '  Get-AppxPackage AngelSerranoDominguez.Litematica3D | Remove-AppxPackage'
Write-Host "  Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Thumbprint -eq '$($cert.Thumbprint)' | Remove-Item"
Write-Host "  Get-ChildItem Cert:\CurrentUser\My | Where-Object Thumbprint -eq '$($cert.Thumbprint)' | Remove-Item"
