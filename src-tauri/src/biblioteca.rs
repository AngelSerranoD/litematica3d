// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Biblioteca local: copia de cada schematic en `<datos>/biblioteca/<uuid>.<ext>`
//! e índice SQLite en `<datos>/biblioteca.sqlite3`. Borrar = mover a la Papelera.
//!
//! Esquema (`PRAGMA user_version`):
//! - v1: tabla `schematics`.
//! - v2: tabla `comprobacion` (ítems marcados como reunidos en la pantalla de
//!   materiales), con borrado en cascada al quitar el schematic.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::ErrorApp;
use crate::schematic::{self, Dimensiones, Formato, Schematic};

/// Versión del esquema de la base de datos (`PRAGMA user_version`).
const VERSION_ESQUEMA: i32 = 2;

/// Longitud máxima de un nombre, en caracteres.
pub const MAX_NOMBRE: usize = 200;

/// Longitud máxima del id de un ítem marcado como reunido, en caracteres.
pub const MAX_ITEM: usize = 200;

const COLUMNAS: &str = "id, nombre, autor, descripcion, formato, ancho, alto, largo, \
     total_bloques, regiones, version_datos, fecha_creacion, fecha_importacion, \
     nombre_original, tamano_archivo";

/// Fila de la biblioteca tal y como la ve el frontend.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entrada {
    pub id: String,
    pub nombre: String,
    pub autor: Option<String>,
    pub descripcion: Option<String>,
    pub formato: Formato,
    pub dimensiones: Dimensiones,
    pub total_bloques: u64,
    pub regiones: u32,
    pub version_datos: Option<i32>,
    /// Milisegundos desde 1970 (fecha guardada en el propio archivo).
    pub fecha_creacion: Option<i64>,
    /// Milisegundos desde 1970.
    pub fecha_importacion: i64,
    pub nombre_original: Option<String>,
    pub tamano_archivo: u64,
}

/// Resumen de una región para el frontend (sin la lista de bloques).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenRegion {
    pub nombre: String,
    pub origen: [i32; 3],
    pub tamano: Dimensiones,
    pub tamano_paleta: u32,
    pub total_bloques: u64,
}

/// Entrada más el detalle leído del archivo copiado.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detalle {
    pub entrada: Entrada,
    pub regiones: Vec<ResumenRegion>,
}

/// Resultado de importar un archivo que no ha fallado.
#[derive(Debug, Clone, PartialEq)]
pub enum Importacion {
    Importado(Entrada),
    /// Ya estaba (mismo SHA-256): no se ha copiado nada.
    Duplicado(Entrada),
}

pub struct Biblioteca {
    dir: PathBuf,
    conn: Mutex<Connection>,
}

impl Biblioteca {
    /// Abre (o crea) la biblioteca dentro de `dir_datos`. Sirve igual para la
    /// app (`app_data_dir`) que para los tests (un directorio temporal).
    pub fn abrir(dir_datos: &Path) -> Result<Self, ErrorApp> {
        let dir = dir_datos.join("biblioteca");
        fs::create_dir_all(&dir)
            .map_err(|e| ErrorApp::io("crear la carpeta de la biblioteca", e))?;
        let mut conn = Connection::open(dir_datos.join("biblioteca.sqlite3"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // Las claves ajenas van por conexión y SQLite las trae apagadas. Se
        // encienden para el `ON DELETE CASCADE` de `comprobacion`; aun así,
        // `eliminar` borra esas filas a mano por si esta compilación de SQLite
        // no las soportara (el PRAGMA se ignoraría sin error).
        conn.pragma_update(None, "foreign_keys", true)?;
        migrar(&mut conn)?;
        Ok(Biblioteca {
            dir,
            conn: Mutex::new(conn),
        })
    }

    /// Carpeta donde se guardan las copias de los archivos.
    pub fn carpeta_archivos(&self) -> &Path {
        &self.dir
    }

    fn conexion(&self) -> MutexGuard<'_, Connection> {
        // Un panic con el candado cogido no deja la conexión inservible.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Importa un archivo: lo valida, lo copia y lo registra. Si algo falla no
    /// queda ni copia ni fila.
    pub fn importar(&self, ruta: &Path) -> Result<Importacion, ErrorApp> {
        let nombre_archivo = ruta
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| ruta.display().to_string());
        let bytes =
            fs::read(ruta).map_err(|e| ErrorApp::io(format!("leer «{nombre_archivo}»"), e))?;
        let hash = sha256_hex(&bytes);
        if let Some(e) = self.buscar_por_hash(&hash)? {
            return Ok(Importacion::Duplicado(e));
        }

        let schem =
            schematic::cargar(&bytes).map_err(|e| ErrorApp::de_schematic(e, &nombre_archivo))?;

        let id = uuid::Uuid::new_v4().to_string();
        let ext = ruta
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .filter(|e| {
                !e.is_empty() && e.len() <= 16 && e.chars().all(|c| c.is_ascii_alphanumeric())
            })
            .unwrap_or_else(|| schem.formato.extension_habitual().to_string());
        let nombre_final = format!("{id}.{ext}");
        let ruta_final = self.dir.join(&nombre_final);
        let ruta_tmp = self.dir.join(format!("{nombre_final}.tmp"));

        let nombre = schem
            .nombre
            .clone()
            .or_else(|| {
                ruta.file_stem()
                    .map(|s| s.to_string_lossy().trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "Sin nombre".into());
        let entrada = entrada_de(
            &schem,
            id,
            recortar(&nombre),
            nombre_archivo,
            bytes.len() as u64,
        );

        if let Err(e) = fs::write(&ruta_tmp, &bytes) {
            let _ = fs::remove_file(&ruta_tmp);
            return Err(ErrorApp::io("copiar el archivo a la biblioteca", e));
        }

        if let Err(e) = self.insertar(&entrada, &nombre_final, &hash) {
            let _ = fs::remove_file(&ruta_tmp);
            // Otra importación simultánea del mismo archivo ganó la carrera.
            if es_violacion_unica(&e)
                && let Some(existente) = self.buscar_por_hash(&hash)?
            {
                return Ok(Importacion::Duplicado(existente));
            }
            return Err(e.into());
        }

        if let Err(e) = fs::rename(&ruta_tmp, &ruta_final) {
            let _ = fs::remove_file(&ruta_tmp);
            let _ = self
                .conexion()
                .execute("DELETE FROM schematics WHERE id = ?1", [&entrada.id]);
            return Err(ErrorApp::io("guardar el archivo en la biblioteca", e));
        }
        Ok(Importacion::Importado(entrada))
    }

    fn insertar(&self, e: &Entrada, ruta: &str, hash: &str) -> rusqlite::Result<()> {
        self.conexion().execute(
            "INSERT INTO schematics (id, nombre, autor, descripcion, formato, ancho, alto, largo, \
             total_bloques, regiones, version_datos, fecha_creacion, fecha_importacion, ruta, \
             nombre_original, tamano_archivo, hash) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                e.id,
                e.nombre,
                e.autor,
                e.descripcion,
                e.formato.como_texto(),
                e.dimensiones.ancho,
                e.dimensiones.alto,
                e.dimensiones.largo,
                e.total_bloques as i64,
                e.regiones,
                e.version_datos,
                e.fecha_creacion,
                e.fecha_importacion,
                ruta,
                e.nombre_original,
                e.tamano_archivo as i64,
                hash,
            ],
        )?;
        Ok(())
    }

    fn buscar_por_hash(&self, hash: &str) -> Result<Option<Entrada>, ErrorApp> {
        let sql = format!("SELECT {COLUMNAS} FROM schematics WHERE hash = ?1");
        Ok(self
            .conexion()
            .query_row(&sql, [hash], fila_a_entrada)
            .optional()?)
    }

    /// Todas las entradas, de la importada más recientemente a la más antigua.
    pub fn listar(&self) -> Result<Vec<Entrada>, ErrorApp> {
        let conn = self.conexion();
        let sql = format!(
            "SELECT {COLUMNAS} FROM schematics ORDER BY fecha_importacion DESC, rowid DESC"
        );
        let mut consulta = conn.prepare(&sql)?;
        let filas = consulta.query_map([], fila_a_entrada)?;
        Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Entrada y ruta interna de un id.
    fn entrada_y_ruta(&self, id: &str) -> Result<(Entrada, String), ErrorApp> {
        let sql = format!("SELECT {COLUMNAS}, ruta FROM schematics WHERE id = ?1");
        self.conexion()
            .query_row(&sql, [id], |f| {
                Ok((fila_a_entrada(f)?, f.get::<_, String>(15)?))
            })
            .optional()?
            .ok_or(ErrorApp::NoEncontrado)
    }

    /// Entrada con el resumen de sus regiones, leído de la copia guardada.
    pub fn obtener(&self, id: &str) -> Result<Detalle, ErrorApp> {
        let (entrada, schem) = self.cargar(id)?;
        let regiones = schem
            .regiones
            .iter()
            .map(|r| ResumenRegion {
                nombre: r.nombre.clone(),
                origen: r.origen,
                tamano: r.tamano,
                tamano_paleta: r.paleta.len() as u32,
                total_bloques: r.total_bloques(),
            })
            .collect();
        Ok(Detalle { entrada, regiones })
    }

    /// Entrada y schematic completo (con bloques) de un id.
    pub fn cargar(&self, id: &str) -> Result<(Entrada, Schematic), ErrorApp> {
        let (entrada, ruta) = self.entrada_y_ruta(id)?;
        let bytes = match fs::read(self.dir.join(&ruta)) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(ErrorApp::ArchivoPerdido {
                    nombre: entrada.nombre,
                });
            }
            Err(e) => return Err(ErrorApp::io(format!("leer «{}»", entrada.nombre), e)),
        };
        let schem =
            schematic::cargar(&bytes).map_err(|e| ErrorApp::de_schematic(e, &entrada.nombre))?;
        Ok((entrada, schem))
    }

    /// Cambia el nombre visible (no el archivo). Vacío o > 200 caracteres: error.
    pub fn renombrar(&self, id: &str, nombre: &str) -> Result<Entrada, ErrorApp> {
        let nombre = nombre.trim();
        if nombre.is_empty() {
            return Err(ErrorApp::NombreInvalido(
                "El nombre no puede estar vacío.".into(),
            ));
        }
        if nombre.chars().count() > MAX_NOMBRE {
            return Err(ErrorApp::NombreInvalido(format!(
                "El nombre no puede pasar de {MAX_NOMBRE} caracteres."
            )));
        }
        let cambiadas = self.conexion().execute(
            "UPDATE schematics SET nombre = ?1 WHERE id = ?2",
            [nombre, id],
        )?;
        if cambiadas == 0 {
            return Err(ErrorApp::NoEncontrado);
        }
        Ok(self.entrada_y_ruta(id)?.0)
    }

    /// Mueve la copia a la Papelera y quita la fila. Si la copia ya no existe,
    /// solo quita la fila.
    pub fn eliminar(&self, id: &str) -> Result<(), ErrorApp> {
        let (entrada, ruta) = self.entrada_y_ruta(id)?;
        let archivo = self.dir.join(&ruta);
        let existe = archivo.try_exists().map_err(|e| {
            ErrorApp::io(format!("comprobar el archivo de «{}»", entrada.nombre), e)
        })?;
        if existe {
            trash::delete(&archivo).map_err(|e| ErrorApp::Papelera {
                nombre: entrada.nombre.clone(),
                detalle: e.to_string(),
            })?;
        }
        let mut conn = self.conexion();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM comprobacion WHERE schematic_id = ?1", [id])?;
        tx.execute("DELETE FROM schematics WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// ¿Están activas las claves ajenas en la conexión? (Para las pruebas.)
    pub fn claves_ajenas_activas(&self) -> Result<bool, ErrorApp> {
        Ok(self
            .conexion()
            .pragma_query_value(None, "foreign_keys", |f| f.get::<_, bool>(0))?)
    }

    /// Ítems marcados como reunidos en un schematic, ordenados por id.
    pub fn comprobacion(&self, id: &str) -> Result<Vec<String>, ErrorApp> {
        let conn = self.conexion();
        existe(&conn, id)?;
        let mut consulta =
            conn.prepare("SELECT item FROM comprobacion WHERE schematic_id = ?1 ORDER BY item")?;
        let filas = consulta.query_map([id], |f| f.get::<_, String>(0))?;
        Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Marca (`reunido = true`) o desmarca un ítem. Es idempotente: marcar
    /// dos veces o desmarcar uno sin marcar no es un error.
    pub fn marcar(&self, id: &str, item: &str, reunido: bool) -> Result<(), ErrorApp> {
        let item = item.trim();
        if item.is_empty() || item.chars().count() > MAX_ITEM {
            return Err(ErrorApp::NombreInvalido(format!(
                "El ítem tiene que tener entre 1 y {MAX_ITEM} caracteres."
            )));
        }
        let conn = self.conexion();
        existe(&conn, id)?;
        if reunido {
            conn.execute(
                "INSERT OR IGNORE INTO comprobacion (schematic_id, item) VALUES (?1, ?2)",
                [id, item],
            )?;
        } else {
            conn.execute(
                "DELETE FROM comprobacion WHERE schematic_id = ?1 AND item = ?2",
                [id, item],
            )?;
        }
        Ok(())
    }
}

/// Crea o actualiza el esquema según `PRAGMA user_version`.
fn migrar(conn: &mut Connection) -> Result<(), ErrorApp> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |f| f.get(0))?;
    if version > VERSION_ESQUEMA {
        return Err(ErrorApp::VersionBiblioteca);
    }
    if version < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE schematics (
                 id TEXT PRIMARY KEY NOT NULL,
                 nombre TEXT NOT NULL,
                 autor TEXT,
                 descripcion TEXT,
                 formato TEXT NOT NULL,
                 ancho INTEGER NOT NULL,
                 alto INTEGER NOT NULL,
                 largo INTEGER NOT NULL,
                 total_bloques INTEGER NOT NULL,
                 regiones INTEGER NOT NULL,
                 version_datos INTEGER,
                 fecha_creacion INTEGER,
                 fecha_importacion INTEGER NOT NULL,
                 ruta TEXT NOT NULL,
                 nombre_original TEXT,
                 tamano_archivo INTEGER NOT NULL,
                 hash TEXT NOT NULL UNIQUE
             );
             CREATE INDEX schematics_fecha ON schematics (fecha_importacion DESC);
             PRAGMA user_version = 1;",
        )?;
        tx.commit()?;
    }
    if version < 2 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE comprobacion (
                 schematic_id TEXT NOT NULL REFERENCES schematics (id) ON DELETE CASCADE,
                 item TEXT NOT NULL,
                 PRIMARY KEY (schematic_id, item)
             ) WITHOUT ROWID;
             PRAGMA user_version = 2;",
        )?;
        tx.commit()?;
    }
    Ok(())
}

/// `NoEncontrado` si no hay ningún schematic con ese id.
fn existe(conn: &Connection, id: &str) -> Result<(), ErrorApp> {
    let hay = conn
        .query_row("SELECT 1 FROM schematics WHERE id = ?1", [id], |_| Ok(()))
        .optional()?;
    hay.ok_or(ErrorApp::NoEncontrado)
}

fn fila_a_entrada(f: &Row<'_>) -> rusqlite::Result<Entrada> {
    let formato_txt: String = f.get(4)?;
    let formato = Formato::desde_texto(&formato_txt).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            format!("formato desconocido: {formato_txt}").into(),
        )
    })?;
    Ok(Entrada {
        id: f.get(0)?,
        nombre: f.get(1)?,
        autor: f.get(2)?,
        descripcion: f.get(3)?,
        formato,
        dimensiones: Dimensiones {
            ancho: f.get(5)?,
            alto: f.get(6)?,
            largo: f.get(7)?,
        },
        total_bloques: f.get::<_, i64>(8)?.max(0) as u64,
        regiones: f.get(9)?,
        version_datos: f.get(10)?,
        fecha_creacion: f.get(11)?,
        fecha_importacion: f.get(12)?,
        nombre_original: f.get(13)?,
        tamano_archivo: f.get::<_, i64>(14)?.max(0) as u64,
    })
}

fn entrada_de(
    s: &Schematic,
    id: String,
    nombre: String,
    nombre_original: String,
    tamano_archivo: u64,
) -> Entrada {
    Entrada {
        id,
        nombre,
        autor: s.autor.clone(),
        descripcion: s.descripcion.clone(),
        formato: s.formato,
        dimensiones: s.dimensiones,
        total_bloques: s.total_bloques(),
        regiones: s.regiones.len() as u32,
        version_datos: s.version_datos,
        fecha_creacion: s.fecha_creacion,
        fecha_importacion: ahora_ms(),
        nombre_original: Some(nombre_original),
        tamano_archivo,
    }
}

fn recortar(nombre: &str) -> String {
    nombre
        .chars()
        .take(MAX_NOMBRE)
        .collect::<String>()
        .trim()
        .to_string()
}

fn es_violacion_unica(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::ConstraintViolation)
}

fn ahora_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// SHA-256 en hexadecimal (minúsculas).
pub fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let resumen = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for b in resumen.iter() {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn hash_conocido() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
