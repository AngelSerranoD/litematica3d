# Textos de anuncio de la v0.1.0

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Textos para que los publique Ángel. Ningún agente publica nada. Cada uno va en
inglés, que es lo que se habla en esas comunidades, con la versión en español
debajo para revisarlo.

Enlaces:

- Demo web: https://www.aserrano.dev/demo/schematic-lab
- Microsoft Store: https://apps.microsoft.com/detail/9P3WWV65DTR6
- Release: https://github.com/AngelSerranoD/schematic-lab/releases/latest
- Vídeo del visor: `docs/anuncio/visor.mp4` (2,7 MB) y `docs/anuncio/visor.gif`
  (3,9 MB). Están grabados de la demo web, sin texturas, porque la demo no
  lleva assets de Mojang. Si puedes, graba 10 segundos de la app instalada
  con un schematic tuyo y las texturas puestas (Win + Alt + R abre la grabación
  de la Barra de juegos): se ve mucho mejor. Los dos archivos no van al
  repositorio (`.gitignore`).

## Antes de publicar: normas que no se han podido leer

Reddit no deja leer las normas sin sesión y el Discord tampoco, así que esto
hay que comprobarlo a mano antes de cada publicación:

- **Las cuatro:** la pestaña de normas (en Reddit, «About» → «Rules»; en
  Discord, el canal `#rules` o similar). Mira si hay una regla de
  autopromoción, un hilo semanal para proyectos o un mínimo de karma o de
  antigüedad de la cuenta.
- **Flair:** en Reddit, si pide flair, elige el de herramienta o recurso
  (suele llamarse «Tool», «Resource», «Showcase» o «Discussion»).
- **r/litematica:** comprueba primero que el subreddit existe y tiene
  actividad. Si está muerto, sáltatelo.
- **r/redstone:** es una comunidad de circuitos. Una herramienta de schematics
  encaja a medias. Si las normas piden que el post sea sobre redstone, no lo
  publiques allí.
- **Discord de Litematica (MasaModding):** invitación
  https://discord.gg/2FgywHj (sacada de la página de Litematica en Modrinth).
  Es el servidor del autor del mod, así que el texto deja claro que no tiene
  relación con él. Busca un canal de proyectos o de autopromoción; si no lo
  hay, pregunta a un moderador antes de publicar.
- **Ritmo:** publica uno al día como mucho, no todos a la vez, y quédate a
  contestar los comentarios la primera hora.

---

## r/technicalminecraft

**Title:** I made a free Windows app to keep a library of schematics, view them in 3D and get the material list in stacks and shulkers

**Body (EN):**

I build a lot from schematics and kept doing the same thing: open the world,
load the schematic in Litematica, check the material list, do the maths for
shulker boxes. So I wrote a small desktop app for the part that doesn't need
the game open.

What it does:

- Imports .litematic, .schem (Sponge), .schematic (WorldEdit) and .nbt
  structure files into a local library with thumbnails.
- 3D viewer with a layer slider, so you can go up one layer at a time.
- Material list as an exact count, in stacks and in shulker boxes
  (16-stackables and unstackables are handled), exportable to CSV.
- Basic block editing: swap a block for a similar one (e.g. oak to spruce
  across the whole build), with undo history.
- Textures come from the Minecraft jar you already have installed. Nothing
  from Mojang is bundled.

You can try it in the browser without installing anything:
https://www.aserrano.dev/demo/schematic-lab

Get it on the Microsoft Store (Windows 10/11, 64-bit, free):
https://apps.microsoft.com/detail/9P3WWV65DTR6
or as a plain installer from GitHub: https://github.com/AngelSerranoD/schematic-lab/releases/latest

Things to know: it's a personal project and Windows only. The GitHub
installer isn't code-signed, so SmartScreen will warn you there ("More info"
→ "Run anyway"); the Store version doesn't have that problem. It works
offline and sends no data.
It's not affiliated with Litematica, Mojang or Microsoft.

Feedback welcome, especially schematics that fail to load or material counts
that look wrong.

**Cuerpo (ES):**

Construyo mucho a partir de schematics y siempre hacía lo mismo: abrir el
mundo, cargar el schematic en Litematica, mirar la lista de materiales y
hacer las cuentas de las shulkers. Así que escribí una pequeña app de
escritorio para la parte que no necesita el juego abierto.

Qué hace:

- Importa archivos .litematic, .schem (Sponge), .schematic (WorldEdit) y .nbt
  de estructura a una biblioteca local con miniaturas.
- Visor 3D con un deslizador de capas, para ir subiendo capa a capa.
- Lista de materiales en número exacto, en stacks y en shulkers (tiene en
  cuenta los ítems de 16 y los que no se apilan), exportable a CSV.
- Edición básica: cambiar un bloque por otro parecido (por ejemplo, roble por
  abeto en toda la construcción), con historial para deshacer.
- Las texturas salen del jar de Minecraft que ya tienes instalado. No incluye
  nada de Mojang.

Se puede probar en el navegador sin instalar nada: (demo)

Descárgala en la Microsoft Store (Windows 10/11, 64 bits, gratis): (Store)
o como instalador normal desde GitHub: (Release)

A tener en cuenta: es un proyecto personal y solo para Windows. El
instalador de GitHub no está firmado, así que ahí SmartScreen avisará («Más
información» → «Ejecutar de todas formas»); la versión de la Store no tiene
ese problema. Funciona sin conexión y no envía datos. No tiene relación con Litematica,
Mojang ni Microsoft.

Se agradecen comentarios, sobre todo schematics que no carguen o recuentos
de materiales que parezcan mal.

---

## r/litematica

**Title:** Desktop viewer for .litematic files with a shulker-box material list (not affiliated with Litematica)

**Body (EN):**

Small tool I made, posting here because most of what I load in it are
.litematic files.

It's a Windows app that keeps your schematics in a library, shows them in 3D
with a layer slider, and turns the material list into stacks and shulker
boxes. It also reads .schem, .schematic and .nbt, and it can save edits back
as .litematic (block swaps, with undo).

It doesn't replace the mod: you still need Litematica to place and build in
game. This is for browsing and planning outside the game.

Browser demo, no install: https://www.aserrano.dev/demo/schematic-lab
Microsoft Store: https://apps.microsoft.com/detail/9P3WWV65DTR6
Installer on GitHub: https://github.com/AngelSerranoD/schematic-lab/releases/latest

Personal project, Windows only. Not made by or affiliated with masa or the Litematica
maintainers. If a file of yours doesn't load, I'd like to know.

**Cuerpo (ES):**

Una herramienta pequeña que he hecho; la publico aquí porque la mayoría de lo
que cargo en ella son archivos .litematic.

Es una app de Windows que guarda tus schematics en una biblioteca, los
muestra en 3D con un deslizador de capas y convierte la lista de materiales
en stacks y shulkers. También lee .schem, .schematic y .nbt, y puede guardar
cambios como .litematic (cambios de bloque, con deshacer).

No sustituye al mod: para colocar y construir en el juego sigue haciendo
falta Litematica. Esto es para mirar y planificar fuera del juego.

Demo en el navegador, sin instalar: (demo)
Microsoft Store: (Store)
Instalador en GitHub: (Release)

Proyecto personal, solo Windows. No lo ha hecho masa ni los mantenedores de Litematica, ni
tiene relación con ellos. Si algún archivo tuyo no carga, me gustaría
saberlo.

---

## r/redstone

Publicar solo si las normas admiten herramientas. Si no, saltárselo.

**Title:** Made a tool to check the material list of a redstone schematic in stacks/shulkers before building

**Body (EN):**

When I download a farm or a contraption as a schematic, the first thing I
want to know is how many shulkers of stuff it needs and how the layers look
before I start. I wrote a Windows app for that.

- Layer-by-layer 3D view (slider on the Y axis), handy for following wiring.
- Click a block to see its ID and position.
- Material list in stacks and shulker boxes, CSV export.
- Reads .litematic, .schem, .schematic and .nbt.

Try it in the browser: https://www.aserrano.dev/demo/schematic-lab
Microsoft Store: https://apps.microsoft.com/detail/9P3WWV65DTR6

Personal project, Windows only. Not affiliated with
Mojang, Microsoft or Litematica.

**Cuerpo (ES):**

Cuando me descargo una granja o un mecanismo como schematic, lo primero que
quiero saber es cuántas shulkers de material hacen falta y cómo son las capas
antes de empezar. Escribí una app de Windows para eso.

- Vista 3D capa a capa (deslizador en el eje Y), útil para seguir el
  cableado.
- Clic en un bloque para ver su ID y su posición.
- Lista de materiales en stacks y shulkers, exportable a CSV.
- Lee .litematic, .schem, .schematic y .nbt.

Pruébalo en el navegador: (demo)
Microsoft Store: (Store)

Proyecto personal, solo Windows. Sin relación con
Mojang, Microsoft ni Litematica.

---

## Discord de Litematica (MasaModding)

Mensaje corto, sin incrustar más de un enlace si el canal lo limita (la demo
es el mejor gancho). Adjunta el vídeo.

**EN:**

Hi! I made a free Windows app for browsing schematics outside the game: a
library with thumbnails, a 3D view with a layer slider, and the material
list in stacks and shulker boxes. It reads .litematic, .schem, .schematic and
.nbt. You can try it in the browser: https://www.aserrano.dev/demo/schematic-lab

It's a personal project and isn't affiliated with Litematica or masa; if this
isn't the right channel, tell me and I'll move or delete it.

**ES:**

¡Hola! He hecho una app gratuita de Windows para mirar schematics fuera del
juego: una biblioteca con miniaturas, una vista 3D con deslizador de capas y
la lista de materiales en stacks y shulkers. Lee .litematic, .schem,
.schematic y .nbt. Se puede probar en el navegador: (demo)

Es un proyecto personal y no tiene relación con Litematica ni con masa; si
este no es el canal adecuado, decídmelo y lo muevo o lo borro.
