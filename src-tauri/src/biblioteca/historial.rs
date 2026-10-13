// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Edición de la copia de la biblioteca con historial para deshacer:
//! aplicar cambios, listar y restaurar versiones anteriores y exportar.
//! El contrato está en `docs/edicion.md`.
//!
//! Lo modificado se guarda siempre como .litematic, sobrescribiendo la copia
//! de la biblioteca (nunca el archivo original del usuario). La versión que
//! había pasa antes a `<datos>/biblioteca/historial/<id>/<versión>.<ext>`, con
//! su formato de entonces, y queda apuntada en la tabla `historial`.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::{
    Biblioteca, Entrada, ahora_ms, es_violacion_unica, existe, id_valido, reemplazar, sha256_hex,
};
use crate::edicion::{self, Cambio, Reemplazo};
use crate::error::ErrorApp;
use crate::schematic::{
    self, Formato, OpcionesLitematic, Schematic, VERSION_DATOS_POR_DEFECTO, escribir_litematic,
};

/// Versiones anteriores que se guardan por schematic. Las que sobran (las
/// más antiguas) van a la Papelera.
pub const MAX_VERSIONES: usize = 20;

/// Qué sustituyó a una versión del historial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Motivo {
    /// Se aplicaron cambios encima.
    Aplicar,
    /// Se restauró otra versión encima.
    Restaurar,
}

impl Motivo {
    fn como_texto(self) -> &'static str {
        match self {
            Motivo::Aplicar => "aplicar",
            Motivo::Restaurar => "restaurar",
        }
    }

    fn desde_texto(texto: &str) -> Option<Self> {
        match texto {
            "aplicar" => Some(Motivo::Aplicar),
            "restaurar" => Some(Motivo::Restaurar),
            _ => None,
        }
    }
}

/// Una versión anterior de un schematic, tal y como la ve el frontend.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    /// Identificador para `restaurar_version`: marca en ms, única dentro del
    /// schematic (casi siempre igual a `fecha`).
    pub version: i64,
    /// Milisegundos desde 1970: cuándo dejó de ser la versión actual.
    pub fecha: i64,
    /// Formato del archivo guardado (puede no ser .litematic).
    pub formato: Formato,
    pub motivo: Motivo,
    /// Posiciones que cambió la operación que la sustituyó (0 al restaurar).
    pub cambios: u64,
    pub total_bloques: u64,
    pub tamano_archivo: u64,
}

/// Resultado de aplicar cambios o restaurar una versión.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edicion {
    /// Entrada al día (formato, tamaño, recuentos; miniatura marcada obsoleta).
    pub entrada: Entrada,
    /// Posiciones que han cambiado de verdad (0 al restaurar).
    pub cambios: u64,
    /// Versión del historial donde quedó lo que había antes (para deshacer
    /// con `restaurar_version`). `None` si no ha cambiado nada.
    pub version: Option<i64>,
}

/// `<dir>/historial/<id>`.
pub(super) fn carpeta(dir: &Path, id: &str) -> PathBuf {
    dir.join("historial").join(id)
}

/// Extensión de una ruta interna si es de las que genera la biblioteca
/// (letras y cifras, sin puntos).
fn extension_de(ruta: &str) -> Option<String> {
    Path::new(ruta)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty() && e.len() <= 16 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_ascii_lowercase)
}

/// Escribe `bytes` en `ruta` y fuerza que lleguen al disco. Si falla, no deja
/// el archivo a medias.
fn escribir_sincronizado(ruta: &Path, bytes: &[u8]) -> io::Result<()> {
    let r = File::create(ruta).and_then(|mut f| {
        f.write_all(bytes)?;
        f.sync_all()
    });
    if r.is_err() {
        let _ = fs::remove_file(ruta);
    }
    r
}

/// Copia `origen` en `destino` a través de un temporal, para que `destino`
/// solo exista si la copia está entera.
fn copiar_entero(origen: &Path, destino: &Path) -> io::Result<()> {
    let bytes = fs::read(origen)?;
    let tmp = destino.with_extension("tmp");
    escribir_sincronizado(&tmp, &bytes)?;
    fs::rename(&tmp, destino).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Prepara el schematic para guardarlo como .litematic y lo serializa. Si el
/// archivo no traía nombre, se le pone el de la biblioteca.
fn a_litematic(s: &mut Schematic, nombre: &str) -> Result<Vec<u8>, ErrorApp> {
    if s.nombre.is_none() {
        s.nombre = Some(nombre.to_string());
    }
    s.version_datos = Some(s.version_datos.unwrap_or(VERSION_DATOS_POR_DEFECTO));
    s.formato = Formato::Litematic;
    escribir_litematic(s, &OpcionesLitematic::default()).map_err(ErrorApp::de_escritura)
}

/// Actualiza la fila del schematic con la copia nueva y marca su miniatura
/// como obsoleta. Si otra entrada ya tiene ese mismo contenido (p. ej. al
/// restaurar el original de un archivo que se volvió a importar después), el
/// hash se guarda como `<hash>:<id>` para no romper su unicidad.
fn actualizar_fila(
    conn: &Connection,
    id: &str,
    s: &Schematic,
    ruta: &str,
    tamano: u64,
    hash: &str,
) -> rusqlite::Result<()> {
    let hacer = |h: &str| {
        conn.execute(
            "UPDATE schematics SET formato = ?1, ancho = ?2, alto = ?3, largo = ?4, \
             total_bloques = ?5, regiones = ?6, version_datos = ?7, ruta = ?8, \
             tamano_archivo = ?9, hash = ?10, miniatura_texturas = NULL WHERE id = ?11",
            params![
                s.formato.como_texto(),
                s.dimensiones.ancho,
                s.dimensiones.alto,
                s.dimensiones.largo,
                s.total_bloques() as i64,
                s.regiones.len() as u32,
                s.version_datos,
                ruta,
                tamano as i64,
                h,
                id,
            ],
        )
    };
    match hacer(hash) {
        Err(e) if es_violacion_unica(&e) => hacer(&format!("{hash}:{id}")),
        r => r,
    }
    .map(|_| ())
}

/// Solo rutas absolutas que acaben en `.litematic` (sin distinguir mayúsculas).
pub fn validar_destino_litematic(ruta: &Path) -> Result<(), ErrorApp> {
    let ext = ruta
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !ruta.is_absolute() || ext.as_deref() != Some("litematic") || ruta.file_name().is_none() {
        return Err(ErrorApp::NombreInvalido(
            "Solo se puede exportar a un archivo .litematic con la ruta completa.".into(),
        ));
    }
    Ok(())
}

impl Biblioteca {
    /// Aplica cambios puntuales y reemplazos (`docs/edicion.md`) a la copia de
    /// la biblioteca. La versión actual pasa al historial y la nueva se guarda
    /// como .litematic. Si no cambia ninguna posición no se toca nada.
    pub fn aplicar_cambios(
        &self,
        id: &str,
        cambios: &[Cambio],
        reemplazos: &[Reemplazo],
    ) -> Result<Edicion, ErrorApp> {
        let _edicion = self.candado_edicion();
        let (entrada, ruta) = self.entrada_y_ruta(id)?;
        let bytes = self.leer_interno(&entrada.nombre, &ruta)?;
        let mut schem =
            schematic::cargar(&bytes).map_err(|e| ErrorApp::de_schematic(e, &entrada.nombre))?;
        drop(bytes);
        let n = edicion::aplicar(&mut schem, cambios, reemplazos)?;
        if n == 0 {
            return Ok(Edicion {
                entrada,
                cambios: 0,
                version: None,
            });
        }
        let nuevos = a_litematic(&mut schem, &entrada.nombre)?;
        self.sustituir(
            id,
            &entrada,
            &ruta,
            &nuevos,
            "litematic",
            &schem,
            Motivo::Aplicar,
            n,
        )
    }

    /// Versiones anteriores de un schematic, de la más reciente a la más antigua.
    pub fn historial(&self, id: &str) -> Result<Vec<Version>, ErrorApp> {
        let conn = self.conexion();
        existe(&conn, id)?;
        let mut consulta = conn.prepare(
            "SELECT version, fecha, formato, motivo, cambios, total_bloques, tamano_archivo \
             FROM historial WHERE schematic_id = ?1 ORDER BY version DESC",
        )?;
        let filas = consulta.query_map([id], |f| {
            let texto = |i: usize, t: String| {
                rusqlite::Error::FromSqlConversionFailure(
                    i,
                    rusqlite::types::Type::Text,
                    format!("valor desconocido: {t}").into(),
                )
            };
            let formato: String = f.get(2)?;
            let motivo: String = f.get(3)?;
            Ok(Version {
                version: f.get(0)?,
                fecha: f.get(1)?,
                formato: Formato::desde_texto(&formato).ok_or_else(|| texto(2, formato.clone()))?,
                motivo: Motivo::desde_texto(&motivo).ok_or_else(|| texto(3, motivo.clone()))?,
                cambios: f.get::<_, i64>(4)?.max(0) as u64,
                total_bloques: f.get::<_, i64>(5)?.max(0) as u64,
                tamano_archivo: f.get::<_, i64>(6)?.max(0) as u64,
            })
        })?;
        Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Vuelve a una versión del historial. La actual pasa antes al historial
    /// (así se puede rehacer restaurándola). La restaurada sigue en el
    /// historial y conserva su formato.
    pub fn restaurar_version(&self, id: &str, version: i64) -> Result<Edicion, ErrorApp> {
        let _edicion = self.candado_edicion();
        let (entrada, ruta) = self.entrada_y_ruta(id)?;
        let ruta_version: String = self
            .conexion()
            .query_row(
                "SELECT ruta FROM historial WHERE schematic_id = ?1 AND version = ?2",
                params![id, version],
                |f| f.get(0),
            )
            .optional()?
            .ok_or(ErrorApp::VersionNoEncontrada)?;
        let bytes = self.leer_interno(&entrada.nombre, &ruta_version)?;
        let schem =
            schematic::cargar(&bytes).map_err(|e| ErrorApp::de_schematic(e, &entrada.nombre))?;
        let ext = extension_de(&ruta_version)
            .unwrap_or_else(|| schem.formato.extension_habitual().to_string());
        self.sustituir(
            id,
            &entrada,
            &ruta,
            &bytes,
            &ext,
            &schem,
            Motivo::Restaurar,
            0,
        )
    }

    /// Escribe la copia actual como .litematic en `destino` (ruta absoluta
    /// elegida por el usuario). Si la copia no es .litematic, se convierte.
    pub fn exportar(&self, id: &str, destino: &Path) -> Result<(), ErrorApp> {
        validar_destino_litematic(destino)?;
        let datos = {
            let _edicion = self.candado_edicion();
            let (entrada, ruta) = self.entrada_y_ruta(id)?;
            let bytes = self.leer_interno(&entrada.nombre, &ruta)?;
            if entrada.formato == Formato::Litematic {
                bytes
            } else {
                let mut schem = schematic::cargar(&bytes)
                    .map_err(|e| ErrorApp::de_schematic(e, &entrada.nombre))?;
                a_litematic(&mut schem, &entrada.nombre)?
            }
        };
        // Temporal junto al destino y renombrar: si falla, el archivo que
        // hubiera allí sigue entero.
        let mut nombre_tmp = destino.file_name().unwrap_or_default().to_os_string();
        nombre_tmp.push(".tmp");
        let tmp = destino.with_file_name(nombre_tmp);
        let guardar = |e| ErrorApp::io("exportar el schematic", e);
        escribir_sincronizado(&tmp, &datos).map_err(guardar)?;
        reemplazar(&tmp, destino).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            guardar(e)
        })
    }

    /// Siguiente marca de versión libre para `id`: ahora, o una más que la
    /// última si el reloj no ha avanzado (o ha ido hacia atrás).
    fn siguiente_version(&self, id: &str, ahora: i64) -> Result<i64, ErrorApp> {
        let ultima: Option<i64> = self.conexion().query_row(
            "SELECT MAX(version) FROM historial WHERE schematic_id = ?1",
            [id],
            |f| f.get(0),
        )?;
        Ok(ultima.map_or(ahora, |u| ahora.max(u + 1)))
    }

    /// Sustituye la copia actual de `id` por `nuevos` (ya validados), guarda
    /// la actual en el historial y pone el índice al día, todo o nada:
    ///
    /// 1. `nuevos` se escriben en `<id>.edicion.tmp` con `sync_all`.
    /// 2. Misma ruta: la actual se copia al historial y el temporal se
    ///    renombra encima (la copia anterior no deja de existir en ningún
    ///    momento). Otra extensión: el temporal pasa a `<id>.<ext nueva>` y la
    ///    actual se mueve al historial.
    /// 3. Las filas (`historial` y `schematics`) van en una transacción que
    ///    solo se confirma si los archivos ya están en su sitio; si algo falla
    ///    se deshace lo hecho y la copia anterior queda como estaba.
    #[allow(clippy::too_many_arguments)]
    fn sustituir(
        &self,
        id: &str,
        actual: &Entrada,
        ruta_actual: &str,
        nuevos: &[u8],
        ext_nueva: &str,
        nuevo: &Schematic,
        motivo: Motivo,
        cambios: u64,
    ) -> Result<Edicion, ErrorApp> {
        if !id_valido(id) {
            return Err(ErrorApp::NoEncontrado);
        }
        let guardar = |e| ErrorApp::io("guardar el schematic", e);
        let archivar = |e| ErrorApp::io("guardar la versión anterior en el historial", e);

        let tmp = self.dir.join(format!("{id}.edicion.tmp"));
        escribir_sincronizado(&tmp, nuevos).map_err(guardar)?;
        let quitar_tmp = || {
            let _ = fs::remove_file(&tmp);
        };

        let dir_historial = carpeta(&self.dir, id);
        if let Err(e) = fs::create_dir_all(&dir_historial) {
            quitar_tmp();
            return Err(archivar(e));
        }
        let ext_actual = extension_de(ruta_actual)
            .unwrap_or_else(|| actual.formato.extension_habitual().to_string());
        let fecha = ahora_ms();
        let mut version = match self.siguiente_version(id, fecha) {
            Ok(v) => v,
            Err(e) => {
                quitar_tmp();
                return Err(e);
            }
        };
        // Un archivo suelto de una operación que no llegó a terminar no se pisa.
        while dir_historial
            .join(format!("{version}.{ext_actual}"))
            .exists()
        {
            version += 1;
        }
        let ruta_historial = format!("historial/{id}/{version}.{ext_actual}");
        let abs_historial = self.dir.join(&ruta_historial);
        let abs_actual = self.dir.join(ruta_actual);
        let ruta_nueva = format!("{id}.{ext_nueva}");
        let abs_nueva = self.dir.join(&ruta_nueva);
        let misma = ruta_nueva == ruta_actual;

        if misma && let Err(e) = copiar_entero(&abs_actual, &abs_historial) {
            quitar_tmp();
            return Err(archivar(e));
        }
        // La copia al historial es nuestra y acaba de crearse: si la operación
        // no sigue adelante, sobra.
        let quitar_copia_historial = || {
            if misma {
                let _ = fs::remove_file(&abs_historial);
            }
        };

        let mut conn = self.conexion();
        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => {
                quitar_tmp();
                quitar_copia_historial();
                return Err(e.into());
            }
        };
        let filas = tx
            .execute(
                "INSERT INTO historial (schematic_id, version, fecha, formato, ruta, motivo, \
                 cambios, total_bloques, tamano_archivo) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    version,
                    fecha,
                    actual.formato.como_texto(),
                    ruta_historial,
                    motivo.como_texto(),
                    cambios as i64,
                    actual.total_bloques as i64,
                    actual.tamano_archivo as i64,
                ],
            )
            .and_then(|_| {
                actualizar_fila(
                    &tx,
                    id,
                    nuevo,
                    &ruta_nueva,
                    nuevos.len() as u64,
                    &sha256_hex(nuevos),
                )
            });
        if let Err(e) = filas {
            drop(tx);
            quitar_tmp();
            quitar_copia_historial();
            return Err(e.into());
        }

        if misma {
            if let Err(e) = reemplazar(&tmp, &abs_actual) {
                drop(tx);
                quitar_tmp();
                quitar_copia_historial();
                return Err(guardar(e));
            }
        } else {
            if let Err(e) = reemplazar(&tmp, &abs_nueva) {
                drop(tx);
                quitar_tmp();
                return Err(guardar(e));
            }
            if let Err(e) = fs::rename(&abs_actual, &abs_historial) {
                drop(tx);
                // El archivo nuevo es nuestro y aún no lo apunta el índice.
                let _ = fs::remove_file(&abs_nueva);
                return Err(archivar(e));
            }
        }

        if let Err(e) = tx.commit() {
            // El índice sigue apuntando a la copia anterior: se devuelve a su sitio.
            let _ = reemplazar(&abs_historial, &abs_actual);
            if !misma {
                let _ = fs::remove_file(&abs_nueva);
            }
            return Err(e.into());
        }
        drop(conn);

        // Lo que pase de MAX_VERSIONES va a la Papelera. Si falla, la edición
        // ya está hecha: se reintentará en la siguiente.
        let _ = self.podar_historial(id, &actual.nombre);

        Ok(Edicion {
            entrada: self.entrada_y_ruta(id)?.0,
            cambios,
            version: Some(version),
        })
    }

    /// Manda a la Papelera las versiones que pasan de [`MAX_VERSIONES`] (las
    /// más antiguas) y quita sus filas. Las filas solo se quitan si la
    /// Papelera ha aceptado los archivos.
    fn podar_historial(&self, id: &str, nombre: &str) -> Result<usize, ErrorApp> {
        let sobran: Vec<(i64, String)> = {
            let conn = self.conexion();
            let mut consulta = conn.prepare(
                "SELECT version, ruta FROM historial WHERE schematic_id = ?1 \
                 ORDER BY version DESC LIMIT -1 OFFSET ?2",
            )?;
            consulta
                .query_map(params![id, MAX_VERSIONES as i64], |f| {
                    Ok((f.get(0)?, f.get(1)?))
                })?
                .collect::<rusqlite::Result<_>>()?
        };
        if sobran.is_empty() {
            return Ok(0);
        }
        let mut archivos = Vec::with_capacity(sobran.len());
        for (_, ruta) in &sobran {
            let archivo = self.dir.join(ruta);
            if archivo.try_exists().unwrap_or(true) {
                archivos.push(archivo);
            }
        }
        if !archivos.is_empty() {
            self.papelera
                .enviar(&archivos)
                .map_err(|detalle| ErrorApp::Papelera {
                    nombre: nombre.to_string(),
                    detalle,
                })?;
        }
        let mut conn = self.conexion();
        let tx = conn.transaction()?;
        for (version, _) in &sobran {
            tx.execute(
                "DELETE FROM historial WHERE schematic_id = ?1 AND version = ?2",
                params![id, version],
            )?;
        }
        tx.commit()?;
        Ok(sobran.len())
    }
}
