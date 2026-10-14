# Distribución: paquete MSIX para la Microsoft Store

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Schematic Lab se distribuye por dos canales:

- **Microsoft Store**, con un paquete **MSIX**. Es el canal principal: la Store
  firma el paquete sin coste, así que no aparece el aviso de SmartScreen.
- **Releases de GitHub**, con el instalador NSIS de siempre (`tauri build`),
  para quien no use la Store. Este documento no lo toca.

Aquí se explica cómo se genera el MSIX, por qué de esta forma y cómo se
comporta la app dentro del paquete.

## 1. Cómo se genera

```powershell
powershell -ExecutionPolicy Bypass -File scripts\empaquetar-msix.ps1
```

Un solo comando. Compila (`npx tauri build --no-bundle`), monta el diseño del
paquete y lo empaqueta. Resultado:

```
src-tauri\target\msix\SchematicLab_<versión>_x64.msix   (≈4,4 MiB, sin firmar)
src-tauri\target\msix\diseno\                            (el mismo contenido, sin comprimir)
```

Todo queda dentro de `src-tauri\target`, que no va a git (`.gitignore` ignora
además `*.msix`, `*.appx` y `*.cer`).

Requisitos: lo mismo que para compilar la app (Node, Rust) más el **Windows
SDK** (trae `makeappx.exe` y `makepri.exe`; el script usa la versión más
reciente que encuentre en `C:\Program Files (x86)\Windows Kits\10\bin`).

Parámetros:

| Parámetro | Por defecto | Para qué |
|---|---|---|
| `-IdentityName` | `AngelSerranoDominguez.SchematicLab` | `Package/Identity/Name`. **Provisional** |
| `-Publisher` | `CN=Angel Serrano Dominguez` | `Package/Identity/Publisher`. **Provisional** |
| `-PublisherDisplayName` | `Ángel Serrano Domínguez` | Nombre del editor que se ve en la Store |
| `-SinCompilar` | — | Reutiliza el exe de `src-tauri\target\release` |

Los tres primeros los da Partner Center al reservar el nombre (Producto →
Identidad del producto). Se cambian en el bloque `param` del principio del
script o se pasan al llamarlo:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\empaquetar-msix.ps1 `
  -IdentityName '12345AngelSerrano.SchematicLab' `
  -Publisher 'CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX'
```

La **versión** sale de `package.json` (la fuente única, como en el resto del
proyecto) con un cuarto número a 0: `0.1.0` → `0.1.0.0`. La Store exige ese 0
final, y cada envío necesita una versión mayor que la anterior.

### Qué hace el script, paso a paso

1. `npx tauri build --no-bundle`: frontend y `SchematicLab.exe` en modo release,
   sin generar el NSIS.
2. Monta `src-tauri\target\msix\diseno` (la borra y la rehace entera cada vez):
   - `SchematicLab.exe`;
   - `LICENSE`, `NOTICE` y `licencias\OFL.txt`, los mismos avisos que lleva el NSIS;
   - `Assets\`, copiado de `src-tauri\msix\Assets`;
   - `AppxManifest.xml`, a partir de la plantilla `src-tauri\msix\AppxManifest.xml`
     con los marcadores `{{...}}` sustituidos.
3. `makepri`: genera `resources.pri`, el índice que usa Windows para elegir el
   logo de cada tamaño (`scale-200`, `targetsize-24`…). Se quita la sección
   `<packaging>` de la configuración por defecto, porque esa sección reparte los
   recursos en varios `.pri` pensados para lotes `.msixbundle`. Con un solo
   `.msix`, los logos `scale-200` se quedarían fuera.
4. `makeappx pack`: crea el `.msix`.

## 2. Vía elegida y lo descartado

Tauri 2 no genera MSIX. Se han mirado tres vías.

**Elegida: `makeappx` del Windows SDK con un `AppxManifest.xml` propio.**
Es la forma documentada por Microsoft de empaquetar a mano una app de
escritorio ([conversión manual](https://learn.microsoft.com/windows/msix/desktop/desktop-to-uwp-manual-conversion)).
El exe de Tauri es un Win32 normal y no necesita nada más que la capacidad
`runFullTrust`. Ventajas:

- es reproducible, porque se lanza con un script, y el manifiesto está
  versionado en git y se revisa como el resto del código;
- no añade dependencias, ya que el SDK ya está instalado y `makeappx` es la
  herramienta de referencia;
- se controla cada línea del manifiesto: asociaciones, logos, versión mínima
  de Windows…

**Descartado:**

- **La guía de Tauri para la Microsoft Store**
  ([v2.tauri.app/distribute/microsoft-store](https://v2.tauri.app/distribute/microsoft-store/)).
  No hace MSIX: sube a la Store el instalador `.exe`/`.msi` como app Win32 «sin
  empaquetar». Ese instalador tiene que ir **firmado con un certificado de
  código propio**, y para un particular eso cuesta dinero (ver «Distribución»
  en el plan). Además obliga a meter el instalador offline de WebView2
  (muchas veces más grande que la app). Justo lo que se quiere evitar.
- **MSIX Packaging Tool.** Convierte un instalador capturando lo que hace al
  instalarse. Es una herramienta gráfica, conviene usarla en una máquina
  virtual limpia y genera un manifiesto que luego hay que retocar. No se puede
  repetir con un comando y no aporta nada sobre un exe que ya es portable.

## 3. El manifiesto

Plantilla: `src-tauri\msix\AppxManifest.xml`. Decisiones:

- **`runFullTrust`** y `EntryPoint="Windows.FullTrustApplication"`: la app
  corre como un Win32 con plena confianza. Solo se virtualiza `AppData` (ver §5).
- **Sin `internetClient`**: la app no usa la red.
- **`MinVersion` 10.0.19041.0** (Windows 10 2004). Las versiones anteriores ya
  no tienen soporte y virtualizan `AppData` de otra forma (copia al escribir).
  `MaxVersionTested` es 10.0.26100.0 (Windows 11 24H2, el SDK instalado).
- **WebView2 no se declara.** No existe como paquete de framework MSIX al que
  apuntar con `PackageDependency`. Viene de serie en Windows 11 y Windows Update
  lo instala en Windows 10. Si faltara, el exe de Tauri lo indica al arrancar.
  En las pruebas, el WebView2 del sistema funcionó dentro del paquete sin
  declarar nada.
- **Asociaciones de archivos**: `.litematic`, `.schem`, `.schematic` y `.nbt`,
  las mismas que el NSIS (`fileAssociations` en `tauri.conf.json`) y que
  `EXTENSIONES` en `src-tauri/src/argumentos.rs`. La app ya sabe abrir el
  archivo que llega por la línea de comandos, también con una instancia ya
  abierta, gracias a `tauri-plugin-single-instance`. Sin asociación, el doble
  clic dejaría de funcionar en la versión de la Store. Instalar el paquete no
  cambia el programa predeterminado: solo lo añade a «Abrir con». La prueba
  `espejos.rs` vigila que las cuatro listas coincidan.
- **Idioma** `es-ES`: la interfaz solo está en español. Si se quiere una ficha
  en inglés en la Store, la ficha se traduce en Partner Center; el idioma del
  manifiesto indica en qué idioma está la app.

### Logos

Están en `src-tauri\msix\Assets` y se generaron desde `src-tauri\icons\icon.png`
(512×512, el icono de la app ya recortado) con `npx tauri icon <icono> -o <carpeta
temporal> -p 16,24,32,44,48,50,88,100,150,256,300`, renombrando cada tamaño:

| Archivo | Píxeles | Uso |
|---|---|---|
| `StoreLogo.scale-100.png` / `.scale-200.png` | 50 / 100 | Logo del paquete (`Properties/Logo`) |
| `Square150x150Logo.scale-100.png` / `.scale-200.png` | 150 / 300 | Icono mediano del menú Inicio |
| `Square44x44Logo.scale-100.png` / `.scale-200.png` | 44 / 88 | Icono pequeño (lista de apps) |
| `Square44x44Logo.targetsize-{16,24,32,48,256}_altform-unplated.png` | 16–256 | Barra de tareas, Explorador, «Abrir con» |

`BackgroundColor="transparent"` y las variantes `altform-unplated` hacen que el
icono salga sin la placa de color de acento. El resto de tamaños que crea
`tauri icon` (Square30/71/89/107/142/284/310, `icon.icns`, Android, iOS) no los
usa el manifiesto y no se guardan.

## 4. Probar el paquete en local

**Sin firmar (modo desarrollador activo).** Se registra el diseño sin comprimir.
Corre con identidad de paquete y la misma virtualización que el `.msix`
instalado:

```powershell
Add-AppxPackage -Register src-tauri\target\msix\diseno\AppxManifest.xml
# ... probar (aparece en el menú Inicio como Schematic Lab) ...
Get-AppxPackage AngelSerranoDominguez.SchematicLab | Remove-AppxPackage
```

Mientras está registrado no se puede regenerar el paquete, porque el script
borra esa carpeta: hay que quitarlo antes.

**El `.msix` firmado**, como lo instalaría un usuario:
`scripts\firmar-msix-prueba.ps1`, en un PowerShell **como administrador**. Crea
un certificado autofirmado con el mismo `Publisher`, firma el paquete, confía
en el certificado (`LocalMachine\TrustedPeople`) y lo instala. Al acabar
enseña los comandos para deshacerlo. Toca almacenes de certificados del
sistema, así que lo lanza Ángel a mano. La Store no necesita nada de esto,
porque firma ella el paquete que se sube.

## 5. Cómo se comporta dentro del paquete

Probado el 14-10-2026 en Windows 11 (26200) con el diseño registrado. Las
llamadas a los comandos de Rust se hicieron desde el propio WebView a través
del protocolo de depuración (ver §7). Los diálogos se rellenaron con mensajes
de Windows a sus controles.

| Qué | Resultado |
|---|---|
| Arranque, visor 3D | Funciona |
| Datos de la biblioteca | Carpetas propias del paquete, distintas de las de NSIS (ver más abajo): biblioteca y `ajustes.json` en `%LOCALAPPDATA%\Packages\<familia del paquete>\LocalState\`, atlas de texturas en `…\LocalCache	exturas\`. Persisten entre arranques y las miniaturas se ven desde el primer arranque. Los datos de WebView2 (preferencias de la interfaz en `localStorage`) siguen en `%LOCALAPPDATA%\com.aserrano.schematiclab`, que Windows redirige a `LocalCache\Local\…` (si la versión NSIS ya la había creado, se usa la real) |
| Desinstalar (`Remove-AppxPackage`) | Borra `Packages\<familia>` entero (biblioteca incluida) y las asociaciones. No deja nada en `%APPDATA%` |
| Carpeta `.minecraft` | Se lee la carpeta real del usuario (`%APPDATA%\.minecraft`). Las versiones se detectan, el atlas se genera y las texturas se ven en el visor. Elegir la carpeta a mano en Ajustes también funciona (`guardar_ajustes` con la ruta) |
| Papelera | Borrar un schematic y el recorte del historial (más de 20 versiones) mandan los archivos a la Papelera del sistema. Allí aparecen con su ruta dentro de la carpeta del paquete |
| Diálogo de abrir (Importar) | Funciona y devuelve la ruta elegida |
| Exportar CSV (diálogo de guardar) | Funciona; el archivo se escribe en la ruta real elegida (fuera de `AppData` no se virtualiza nada) |
| Asociación de archivos | Abrir un `.litematic` con la asociación del paquete arranca la app y lo importa. Con la app ya abierta, la instancia abierta recibe el archivo |
| Arrastrar y soltar | **No comprobado**: necesita un arrastre real del Explorador. El paquete corre con el mismo nivel de integridad que el Explorador, así que no se espera bloqueo. Revisar a mano |
| WebView2 | El del sistema, sin declarar dependencia |

### Biblioteca propia de la versión de la Store

Decidido con Ángel (14-10-2026): la versión empaquetada usa su **propia**
biblioteca, separada de la de NSIS. Importar la biblioteca de NSIS queda para
más adelante.

Al arrancar, `src-tauri/src/paquete.rs` pregunta a Windows si el proceso tiene
identidad de paquete (`GetCurrentPackageFamilyName`). Si la tiene, la app usa
las carpetas del propio paquete en vez de `app_data_dir`, `app_config_dir` y
`app_cache_dir`:

| | NSIS / `cargo run` | Store (MSIX) |
|---|---|---|
| Biblioteca y `ajustes.json` | `%APPDATA%\com.aserrano.schematiclab\` | `%LOCALAPPDATA%\Packages\<familia>\LocalState\` |
| Atlas de texturas | `%LOCALAPPDATA%\com.aserrano.schematiclab	exturas\` | `%LOCALAPPDATA%\Packages\<familia>\LocalCache	exturas\` |

Por qué así y no dejando que Windows virtualice `%APPDATA%`:

- **Nada compartido con NSIS.** Windows solo redirige lo que el paquete crea
  nuevo en `AppData`; lo que **ya existía** lo abre real. En la prueba, el
  paquete con el identificador de siempre abrió la biblioteca real de la
  versión NSIS. Si hubiera escrito en ella, el SQLite real habría acabado
  apuntando a copias guardadas en la carpeta privada, que desaparecen al
  desinstalar. `LocalState` es la carpeta de datos que Windows da a cada
  paquete: no se virtualiza, la ruta es la real y se borra al desinstalar.
- **Miniaturas desde el primer arranque.** Con `%APPDATA%` virtualizado, el
  protocolo `asset` rechazaba las miniaturas hasta reiniciar. Tauri compara
  las rutas canonicalizadas, que dentro del paquete son las físicas de
  `LocalCache`, y solo resuelve el patrón del alcance si la carpeta ya existe
  al construir la app. Con `LocalState` no hay redirección. El alcance de
  `tauri.conf.json` lleva un segundo patrón,
  `$LOCALDATA/Packages/*/LocalState/biblioteca/*.miniatura.png`. El `*` es la
  familia del paquete, que depende de la identidad que dé Partner Center.

Comprobado registrado: con una biblioteca «NSIS» presente (la del identificador
de pruebas, creada con el exe sin empaquetar), la versión empaquetada arrancó
con la biblioteca vacía y creó la suya en `LocalState`. La de NSIS quedó intacta
(misma suma MD5). Al desinstalar, Windows borró `Packages\<familia>` entero.

Pruebas: `paquete.rs` (rutas del paquete; fuera del paquete no hay familia) y
`espejos.rs` (los dos patrones del alcance usan `biblioteca::CARPETA_COPIAS`).

### Convivencia con la versión NSIS en el mismo equipo

Se pueden tener las dos instaladas. Cada una tiene su biblioteca y sus ajustes,
y desinstalar una no toca los datos de la otra. Lo único compartido son los
datos de WebView2, donde la interfaz guarda tres preferencias (tema, vista de
la biblioteca y pestaña del panel). Quien pase de NSIS a la Store tiene que
importar de nuevo sus schematics.

## 6. Guía para la Store

Pasos que da Ángel: los agentes preparan los archivos, pero no crean la
cuenta ni envían nada. Los nombres de los menús de Partner Center cambian a
menudo; si alguno no coincide, se busca el equivalente.

**Antes de empezar**

1. Publicar el portfolio con la página de privacidad
   (`https://www.aserrano.dev/privacidad/schematic-lab`). La Store la pide como
   URL pública, así que tiene que estar en línea antes del envío.
2. Probar a mano arrastrar y soltar con el paquete registrado (§4). Es lo único
   que no se pudo comprobar sin ratón.

**Cuenta y nombre**

3. Entrar en <https://storedeveloper.microsoft.com> con la cuenta Microsoft y
   crear la cuenta de desarrollador **individual** (gratuita). Se verifica
   la identidad; puede tardar unos días.
4. En Partner Center → *Aplicaciones y juegos* → *Nueva aplicación* → *MSIX
   or PWA app*, reservar el nombre **Schematic Lab**. Si está cogido, parar y
   decidir otro antes de seguir: cambia el `DisplayName` del manifiesto.
   La identidad definitiva del paquete (`Package/Identity/Name` y
   `Publisher`) la da Partner Center al reservarlo; la de los scripts
   (`AngelSerranoDominguez.SchematicLab`) es provisional.
5. En la app reservada → *Administración de productos* → *Identidad del
   producto*, copiar tres valores:
   - `Package/Identity/Name` → `-IdentityName`
   - `Package/Identity/Publisher` (`CN=…` con un GUID) → `-Publisher`
   - `Package/Properties/PublisherDisplayName` → `-PublisherDisplayName`

**Paquete**

6. Ponerlos como valores por defecto en el bloque `param` de
   `scripts\empaquetar-msix.ps1` (así se queda escrito en el repo) y regenerar:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts\empaquetar-msix.ps1
   ```

   Sale `src-tauri\target\msix\SchematicLab_<versión>_x64.msix`, **sin
   firmar**: la Store lo firma al publicarlo. La familia del paquete cambia con
   la identidad nueva; las miniaturas siguen funcionando porque el alcance del
   protocolo `asset` usa un comodín (§5).

**Envío** (Partner Center → la app → *Iniciar el envío*)

7. *Precios y disponibilidad*: gratis, todos los mercados, visible en la Store.
8. *Propiedades*: categoría **Utilidades y herramientas**; URL de privacidad la
   del paso 1; sin conexión a Internet obligatoria.
9. *Clasificación por edades*: el cuestionario IARC. La app no tiene
   contenido generado por usuarios que se comparta, ni chat, ni compras: sale
   la clasificación más baja.
10. *Paquetes*: subir el `.msix` del paso 6. Partner Center lo valida al
    momento; si la identidad no coincide con la reservada, avisa aquí.
11. *Descripción de Store*: añadir el idioma español y el inglés y copiar de
    `docs/store/ficha.es.md` y `docs/store/ficha.en.md` la descripción, la
    descripción corta, las características, los términos de búsqueda y el aviso
    de marca. Capturas: las de `docs/store/capturas/` (1920×1080). Logo de
    Store: lo saca del paquete. Comprobar en el formulario los límites de
    caracteres anotados en las fichas.
12. *Opciones de envío*: en las notas para certificación, explicar que es una
    app sin conexión y que, para ver las texturas, necesita tener Minecraft
    Java instalado (lee el jar del juego de la carpeta `.minecraft`).
13. *Enviar a la Store*. La certificación suele tardar de unos días a una
    semana. Si la rechazan, el informe dice qué política falla.

**Cuando esté publicada**

14. Añadir el enlace de la Store al README, a la Release de GitHub y a la
    ficha del portfolio.

**Versiones siguientes**: subir la versión en `package.json` (el manifiesto la
toma de ahí como `x.y.z.0`), regenerar con el script y crear un envío nuevo
en Partner Center con el paquete nuevo.

## 7. Notas para depurar dentro del paquete

- `Invoke-CommandInDesktopPackage -PackageFamilyName <familia> -AppId SchematicLab
  -Command cmd.exe -Args '/c ...'` abre un proceso dentro del contenedor del
  paquete. Sirve para ver qué ve la app en `%APPDATA%`. Las variables de
  entorno del PowerShell que lo lanza **no** llegan; hay que fijarlas dentro
  del `cmd /c set ...&& start "" SchematicLab.exe`.
- Con `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`, el
  WebView expone el protocolo de depuración de Chromium en
  `http://127.0.0.1:9333/json/list`. Desde ahí se pueden llamar los comandos
  (`window.__TAURI_INTERNALS__.invoke(...)`) y hacer capturas de pantalla.
- `IApplicationActivationManager::ActivateForFile` no sirve para apps de
  plena confianza: da el error «no es compatible con el contrato». Para probar
  la asociación sin cambiar el programa predeterminado, se usa `ShellExecuteEx`
  con `lpClass` = la ProgID `AppX…` del paquete. Está en
  `HKCU\Software\Classes\.litematic\OpenWithProgids`.
