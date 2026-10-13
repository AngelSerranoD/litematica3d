// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Errores de la aplicación. Llegan al frontend como
//! `{ "tipo": "...", "mensaje": "texto en español para el usuario" }`.

use std::io;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::schematic::ErrorSchematic;

#[derive(Debug, thiserror::Error)]
pub enum ErrorApp {
    #[error("{}", con_detalle(format!("El archivo «{archivo}» está dañado o incompleto"), detalle))]
    ArchivoCorrupto {
        archivo: String,
        detalle: Option<String>,
    },

    #[error("El archivo «{archivo}» es demasiado grande: {detalle}.")]
    DemasiadoGrande { archivo: String, detalle: String },

    #[error(
        "El archivo «{archivo}» no es un schematic que se pueda abrir (.litematic, .schem, .schematic o .nbt)."
    )]
    FormatoDesconocido { archivo: String },

    #[error("Ese schematic no está en la biblioteca.")]
    NoEncontrado,

    #[error("El schematic no tiene ninguna región llamada «{nombre}».")]
    RegionNoEncontrada { nombre: String },

    #[error(
        "No se encuentra el archivo de «{nombre}» dentro de la biblioteca; puede que se haya borrado a mano."
    )]
    ArchivoPerdido { nombre: String },

    #[error("{0}")]
    NombreInvalido(String),

    #[error("No se ha podido {accion}: {}.", motivo_io(fuente))]
    Io {
        accion: String,
        #[source]
        fuente: io::Error,
    },

    #[error("Ha fallado la base de datos de la biblioteca.")]
    BaseDatos(#[from] rusqlite::Error),

    #[error("No se ha podido mover «{nombre}» a la Papelera.")]
    Papelera { nombre: String, detalle: String },

    #[error(
        "La biblioteca la creó una versión más nueva de Litematica3D; actualiza la aplicación."
    )]
    VersionBiblioteca,

    /// La carpeta `.minecraft` elegida en los ajustes no vale.
    #[error("{0}")]
    CarpetaMinecraft(String),

    #[error("Error interno de la aplicación.")]
    Interno(String),
}

fn con_detalle(base: String, detalle: &Option<String>) -> String {
    match detalle {
        Some(d) => format!("{base}: {d}."),
        None => format!("{base}."),
    }
}

/// Motivo de un error de E/S en español (el texto del sistema puede venir en inglés).
fn motivo_io(e: &io::Error) -> &'static str {
    match e.kind() {
        io::ErrorKind::NotFound => "no existe",
        io::ErrorKind::PermissionDenied => "no hay permiso",
        io::ErrorKind::AlreadyExists => "ya existe",
        io::ErrorKind::StorageFull => "el disco está lleno",
        io::ErrorKind::IsADirectory => "es una carpeta, no un archivo",
        io::ErrorKind::InvalidFilename => "el nombre no es válido",
        _ => "error del sistema de archivos",
    }
}

impl ErrorApp {
    /// Identificador estable para el frontend.
    pub fn tipo(&self) -> &'static str {
        match self {
            ErrorApp::ArchivoCorrupto { .. } => "archivoCorrupto",
            ErrorApp::DemasiadoGrande { .. } => "demasiadoGrande",
            ErrorApp::FormatoDesconocido { .. } => "formatoDesconocido",
            ErrorApp::NoEncontrado => "noEncontrado",
            ErrorApp::RegionNoEncontrada { .. } => "regionNoEncontrada",
            ErrorApp::ArchivoPerdido { .. } => "archivoPerdido",
            ErrorApp::NombreInvalido(_) => "nombreInvalido",
            ErrorApp::Io { .. } => "io",
            ErrorApp::BaseDatos(_) => "baseDatos",
            ErrorApp::Papelera { .. } => "papelera",
            ErrorApp::VersionBiblioteca => "versionBiblioteca",
            ErrorApp::CarpetaMinecraft(_) => "carpetaMinecraft",
            ErrorApp::Interno(_) => "interno",
        }
    }

    pub fn io(accion: impl Into<String>, fuente: io::Error) -> Self {
        ErrorApp::Io {
            accion: accion.into(),
            fuente,
        }
    }

    /// Traduce un error del lector de schematics, con el nombre del archivo.
    pub fn de_schematic(e: ErrorSchematic, archivo: &str) -> Self {
        let archivo = archivo.to_string();
        match e {
            ErrorSchematic::Corrupto { .. } => ErrorApp::ArchivoCorrupto {
                archivo,
                detalle: None,
            },
            ErrorSchematic::Invalido(d) => ErrorApp::ArchivoCorrupto {
                archivo,
                detalle: Some(d),
            },
            ErrorSchematic::DemasiadoGrande(detalle) => {
                ErrorApp::DemasiadoGrande { archivo, detalle }
            }
            ErrorSchematic::FormatoDesconocido => ErrorApp::FormatoDesconocido { archivo },
        }
    }
}

impl Serialize for ErrorApp {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut e = s.serialize_struct("ErrorApp", 2)?;
        e.serialize_field("tipo", self.tipo())?;
        e.serialize_field("mensaje", &self.to_string())?;
        e.end()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn se_serializa_con_tipo_y_mensaje() {
        let e = ErrorApp::de_schematic(ErrorSchematic::corrupto("eof"), "x.litematic");
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["tipo"], "archivoCorrupto");
        assert_eq!(
            json["mensaje"],
            "El archivo «x.litematic» está dañado o incompleto."
        );
    }
}
