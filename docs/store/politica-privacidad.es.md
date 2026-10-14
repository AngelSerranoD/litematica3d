# Política de privacidad de Schematic Lab

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Última actualización: 14 de octubre de 2026. Publicada en
https://www.aserrano.dev/privacidad/schematic-lab (la página del portfolio
lleva el mismo texto; si se cambia uno, hay que cambiar el otro).

Schematic Lab es una aplicación de escritorio para Windows, de Ángel Serrano
Domínguez, que muestra en 3D schematics de Minecraft y calcula sus materiales.

## Qué datos recoge

Ninguno. La aplicación no envía datos a ningún servidor, no tiene cuentas, no
incluye telemetría, analítica ni publicidad y funciona sin conexión. No hay
ningún servidor mío detrás de ella.

## Qué lee y qué guarda en tu equipo

- Los archivos de schematics (.litematic, .schem, .schematic, .nbt) que
  eliges importar. Se copian a la biblioteca de la app; el original no se
  modifica.
- La carpeta .minecraft (por defecto %APPDATA%\.minecraft, o la que elijas en
  Ajustes) para localizar las versiones de Minecraft instaladas, y el jar de la
  versión elegida para leer sus texturas y los nombres de los ítems. Las
  texturas no salen de tu equipo ni se redistribuyen.
- Su biblioteca y sus ajustes: copias de los schematics, el historial de
  versiones, las miniaturas, un índice SQLite y las preferencias (tema, carpeta
  de Minecraft, versión de texturas, lo ya reunido en las listas de materiales).
  En la versión de la Microsoft Store se guardan en la carpeta privada que
  Windows reserva para la app (%LOCALAPPDATA%\Packages\…\LocalState, y la caché
  de texturas en …\LocalCache). En la versión del instalador de GitHub, en
  %APPDATA%\com.aserrano.schematiclab\, y la caché de texturas en
  %LOCALAPPDATA%\com.aserrano.schematiclab\.
- Los archivos que exportas (CSV de materiales, schematics), en el sitio que
  eliges.

Todo queda en tu equipo. Eliminar un schematic desde la app lo manda a la
Papelera de Windows. Para borrar todo lo que guarda la app: en la versión de la
Microsoft Store basta desinstalarla, porque Windows borra su carpeta; en la del
instalador de GitHub, desinstálala y borra las carpetas
%APPDATA%\com.aserrano.schematiclab\ y
%LOCALAPPDATA%\com.aserrano.schematiclab\.

## Terceros

La aplicación no comparte datos con terceros. Usa el componente WebView2 de
Microsoft, que forma parte de Windows y se rige por las condiciones de
Microsoft; la app no le envía ni le pide nada de red. Minecraft es una marca de
Mojang AB / Microsoft. No es un producto oficial de Minecraft. No está aprobado por Mojang ni Microsoft ni asociado con ellos. Tampoco está afiliado a Litematica ni a su autor.

## Menores

La aplicación no recoge datos de nadie, tampoco de menores.

## Cambios y contacto

Si esta política cambia, se actualizará esta página con su fecha. Para
cualquier duda: angelsd7704@gmail.com.
