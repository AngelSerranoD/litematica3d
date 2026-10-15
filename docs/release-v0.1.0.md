Primera versión pública de Schematic Lab.

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Schematic Lab es una app de escritorio para Windows que importa schematics de
Minecraft (`.litematic`, `.schem`, `.schematic`, `.nbt`), los guarda en una
biblioteca local, los muestra en 3D y calcula los materiales: el número
exacto, en stacks y en shulkers. Funciona sin conexión y no envía datos.

- Biblioteca con miniaturas, búsqueda y arrastrar y soltar para importar.
- Visor 3D con corte por capas y texturas sacadas del Minecraft que ya
  tengas instalado (sin él, colores planos).
- Lista de materiales exportable a CSV.
- Edición de bloques con reemplazo de bloques parecidos e historial para
  deshacer.

Pruébala sin instalar en la demo web: https://www.aserrano.dev/demo/schematic-lab

## Requisitos

- Windows 10 u 11 de 64 bits.
- WebView2: viene con Windows 11; si falta, el instalador lo descarga.

## Instalar

La forma más cómoda es la Microsoft Store, sin aviso de SmartScreen y con
actualizaciones automáticas: https://apps.microsoft.com/detail/9P3WWV65DTR6

Si prefieres no usar la Store, descarga `SchematicLab_0.1.0_x64-setup.exe` y ábrelo.

El instalador no está firmado con un certificado de pago, así que Windows
puede mostrar «Windows protegió su PC» (SmartScreen). Pulsa «Más información»
y luego «Ejecutar de todas formas».

Para comprobar el archivo, compara su hash con `SHA256SUMS.txt`:

```powershell
Get-FileHash .\SchematicLab_0.1.0_x64-setup.exe -Algorithm SHA256
```

## Aviso legal

No es un producto oficial de Minecraft. No está aprobado por Mojang ni
Microsoft ni asociado con ellos. Tampoco está afiliado a Litematica ni a su
autor.

Software propietario. © 2026 Ángel Serrano Domínguez. Todos los derechos
reservados.
