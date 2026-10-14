# Schematic Lab privacy policy

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Last updated: 14 October 2026. Published at
https://www.aserrano.dev/privacidad/schematic-lab (the portfolio page carries
the same text; if one changes, change the other).

Schematic Lab is a Windows desktop app by Ángel Serrano Domínguez that shows
Minecraft schematics in 3D and works out their materials.

## What data it collects

None. The app sends no data to any server, has no accounts, includes no
telemetry, analytics or advertising, and works offline. There is no server of
mine behind it.

## What it reads and stores on your computer

- The schematic files (.litematic, .schem, .schematic, .nbt) you choose to
  import. They are copied into the app's library; the original is not modified.
- The .minecraft folder (by default %APPDATA%\.minecraft, or the one you pick in
  Settings) to find the installed Minecraft versions, and the jar of the chosen
  version to read its textures and item names. Textures never leave your
  computer and are not redistributed.
- Its library and settings: copies of the schematics, the version history,
  thumbnails, a SQLite index and the preferences (theme, Minecraft folder,
  texture version, what you have ticked off in materials lists). In the
  Microsoft Store version they are kept in the private folder Windows reserves
  for the app (%LOCALAPPDATA%\Packages\…\LocalState, with the texture cache in
  …\LocalCache). In the GitHub installer version, in
  %APPDATA%\com.aserrano.schematiclab\, with the texture cache in
  %LOCALAPPDATA%\com.aserrano.schematiclab\.
- The files you export (materials CSV, schematics), wherever you choose.

Everything stays on your computer. Deleting a schematic from the app sends it to
the Windows Recycle Bin. To remove everything the app stores: in the Microsoft
Store version, just uninstall it, because Windows deletes its folder; in the
GitHub installer version, uninstall it and delete the folders
%APPDATA%\com.aserrano.schematiclab\ and
%LOCALAPPDATA%\com.aserrano.schematiclab\.

## Third parties

The app shares no data with third parties. It uses Microsoft's WebView2
component, which is part of Windows and is governed by Microsoft's terms; the
app sends it nothing over the network and asks nothing of it. Minecraft is a
trademark of Mojang AB / Microsoft. Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft. Not affiliated with Litematica or its author.

## Children

The app collects no data from anyone, children included.

## Changes and contact

If this policy changes, this page will be updated with its date. For any
question: angelsd7704@gmail.com.
