# Schematic Lab

Aplicación de escritorio para Windows que importa schematics de Minecraft
(`.litematic`, `.schem`, `.schematic`, `.nbt`), los guarda en una biblioteca
local, los muestra en un visor 3D y calcula los materiales necesarios en tres
modos: exacto, en stacks y en shulkers. Funciona sin conexión.

Se puede probar sin instalar en la [demo web](https://www.aserrano.dev/demo/schematic-lab).

## Instalar

Requisitos: Windows 10 u 11 de 64 bits.

**Microsoft Store (recomendado):** [Schematic Lab en la Store](https://apps.microsoft.com/detail/9P3WWV65DTR6).
Está firmada por Microsoft, así que no sale el aviso de SmartScreen, y se
actualiza sola.

**Instalador de GitHub:** para quien no use la Store. Necesita WebView2, que
viene con Windows 11; si falta, el instalador lo descarga.

1. Descarga `SchematicLab_<versión>_x64-setup.exe` de la
   [última Release](https://github.com/AngelSerranoD/schematic-lab/releases/latest).
2. Ábrelo. Windows puede mostrar «Windows protegió su PC» (SmartScreen)
   porque el instalador no está firmado con un certificado de pago. Pulsa
   «Más información» y luego «Ejecutar de todas formas».
3. Si quieres comprobar que el archivo es el publicado, compara su hash con
   el de `SHA256SUMS.txt` de la Release:

   ```powershell
   Get-FileHash .\SchematicLab_0.1.0_x64-setup.exe -Algorithm SHA256
   ```

   SHA-256 de la versión 0.1.0:
   `78d073c65e26b691b886809f4616f6ec24db1f6de527e43895013c9e613ccf08`

La versión de la Store guarda su biblioteca aparte de la del instalador de
GitHub: si cambias de una a otra, vuelve a importar tus schematics.

No es un producto oficial de Minecraft. No está aprobado por Mojang ni
Microsoft ni asociado con ellos. Tampoco está afiliado a Litematica ni a su
autor.

## Stack

- Tauri 2 (Rust) como contenedor de escritorio
- React 19 + TypeScript + Vite
- Three.js con @react-three/fiber y drei para el visor 3D
- Motion (animaciones) y Zustand (estado)
- Núcleo en Rust: parsers propios de los cuatro formatos sobre `fastnbt`
  (detección por contenido, no por extensión) y biblioteca en
  `%APPDATA%\com.aserrano.schematiclab\` (copias de los archivos e índice
  SQLite). Eliminar manda el archivo a la Papelera de Windows.
- Edición: cambios puntuales y reemplazos sobre la copia de la biblioteca,
  guardada siempre como `.litematic`, con historial de 20 versiones (el
  número vive en `MAX_VERSIONES`) para deshacer y exportación. Contrato en [docs/edicion.md](docs/edicion.md).
- Seguridad: la CSP de `tauri.conf.json` solo permite recursos propios. La
  excepción es `style-src 'unsafe-inline'` (con
  `dangerousDisableAssetCspModification` para que Tauri no la anule con
  nonces), porque Motion y React escriben estilos en línea al animar. No se
  carga nada de fuera ni se ejecuta script en línea.
- Visor 3D: mallado en Rust por chunks de 16³ (caras ocultas, greedy meshing,
  modelos aproximados) enviado en binario. Contrato en
  [docs/visor.md](docs/visor.md).

## Requisitos

- Node.js (LTS reciente) y npm
- Rust (rustup, toolchain estable)
- Microsoft C++ Build Tools (carga de trabajo «Desarrollo para el escritorio con C++»)
- WebView2 (incluido en Windows 11)

## Comandos

```bash
npm install            # instala dependencias
npm run tauri dev      # modo desarrollo
npm run tauri build    # genera el instalador
npm run build          # solo comprueba tipos y compila el frontend
npm test               # tests del frontend (Vitest)
cd src-tauri && cargo test   # tests del núcleo (formatos, biblioteca y malla)
```

## Estructura

Frontend (`src/`):

- `api/`: funciones que llaman a los comandos de Rust y sus tipos.
- `estado/` y `estado/edicion/`: almacenes Zustand (biblioteca, visor, edición).
- `logica/`: funciones puras sin React (formato, materiales, tiempo, elemento de biblioteca).
- `componentes/`: piezas de interfaz reutilizables.
- `vistas/` y `vistas/biblioteca/`: pantallas (biblioteca y schematic).
- `estilos/`: tokens, base y movimiento.

Núcleo (`src-tauri/src/`):

- `comandos/`: comandos Tauri, una capa fina sobre el resto.
- `biblioteca/`: índice SQLite, archivos, historial y miniaturas.
- `schematic/`: parsers y escritores de los cuatro formatos.
- `malla/`: mallado del visor y su binario.
- `texturas/`: atlas desde el jar de Minecraft instalado.
- `materiales/`: lista de materiales y sus modos.
- `parecidos/`: bloques parecidos para reemplazar.
- `espejos.rs`: pruebas que vigilan los valores repetidos entre Rust y TS.

Contratos detallados: [edición](docs/edicion.md), [visor](docs/visor.md),
[materiales](docs/materiales.md) y [miniaturas](docs/miniaturas.md).

## Aviso legal

No es un producto oficial de Minecraft. No está aprobado por Mojang ni Microsoft ni asociado con ellos. Tampoco está afiliado a Litematica ni a su autor.

Minecraft es una marca de Mojang AB / Microsoft. La app
no incluye ni redistribuye assets de Mojang: lee las texturas del Minecraft que
ya tengas instalado.

## Licencia

Software propietario. © 2026 Ángel Serrano Domínguez. Todos los derechos
reservados. Ver [LICENSE](LICENSE) y [NOTICE](NOTICE).
