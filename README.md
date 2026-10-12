# Litematica3D

Aplicación de escritorio para Windows que importa schematics de Minecraft
(`.litematic`, `.schem`, `.schematic`, `.nbt`), los guarda en una biblioteca
local, los muestra en un visor 3D y calcula los materiales necesarios en tres
modos: exacto, en stacks y en shulkers. Funciona sin conexión.

## Stack

- Tauri 2 (Rust) como contenedor de escritorio
- React 19 + TypeScript + Vite
- Motion (animaciones) y Zustand (estado)

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
```

## Licencia

Software propietario. © 2026 Ángel Serrano Domínguez. Todos los derechos
reservados. Ver [LICENSE](LICENSE) y [NOTICE](NOTICE).
