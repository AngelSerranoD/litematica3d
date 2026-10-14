// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Miniaturas isométricas de la biblioteca: validación del PNG y de la clave
//! de texturas, y guardado junto a la copia del schematic (ver
//! `docs/miniaturas.md`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::params;

use super::archivos::{id_valido, reemplazar};
use super::{
    Biblioteca, Entrada, LADO_MINIATURA, MAX_BYTES_MINIATURA, MAX_CLAVE_TEXTURAS, ahora_ms, existe,
};
use crate::error::ErrorApp;

/// `<dir>/<id>.miniatura.png`. La extensión de las copias nunca lleva puntos,
/// así que no puede chocar con el archivo del schematic.
pub(super) fn ruta_miniatura(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.miniatura.png"))
}

/// Clave de texturas: de 1 a 100 caracteres ASCII visibles.
fn validar_clave_texturas(clave: &str) -> Result<(), ErrorApp> {
    if clave.is_empty()
        || clave.len() > MAX_CLAVE_TEXTURAS
        || !clave.chars().all(|c| c.is_ascii_graphic())
    {
        return Err(ErrorApp::MiniaturaInvalida(
            "la clave de texturas no es válida".into(),
        ));
    }
    Ok(())
}

/// Comprueba que `bytes` sea un PNG entero, cuadrado y de un tamaño razonable.
pub fn validar_png(bytes: &[u8]) -> Result<(), ErrorApp> {
    const FIRMA: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let invalida = |m: &str| ErrorApp::MiniaturaInvalida(m.into());
    if bytes.len() > MAX_BYTES_MINIATURA {
        return Err(invalida("pasa de 4 MiB"));
    }
    if !bytes.starts_with(&FIRMA) {
        return Err(invalida("no es un PNG"));
    }
    let mut lector = png::Decoder::new(io::Cursor::new(bytes))
        .read_info()
        .map_err(|_| invalida("el PNG está dañado"))?;
    let (ancho, alto) = {
        let info = lector.info();
        (info.width, info.height)
    };
    if ancho != alto || !LADO_MINIATURA.contains(&ancho) {
        return Err(invalida(
            "tiene que ser cuadrada, de 64 a 1024 píxeles de lado",
        ));
    }
    // Se decodifica entera: así no pasa un PNG cortado o con datos basura.
    let tamano = lector
        .output_buffer_size()
        .ok_or_else(|| invalida("el PNG está dañado"))?;
    let mut buf = vec![0; tamano];
    lector
        .next_frame(&mut buf)
        .map_err(|_| invalida("el PNG está dañado"))?;
    Ok(())
}

impl Biblioteca {
    /// Ruta del PNG de la miniatura de un id (exista o no).
    pub fn ruta_miniatura(&self, id: &str) -> PathBuf {
        ruta_miniatura(&self.dir, id)
    }

    /// Guarda la miniatura de un schematic: valida el PNG y la clave de
    /// texturas, lo escribe como `<id>.miniatura.png` (el nombre sale del id,
    /// nunca de una ruta del frontend) y apunta fecha y clave en el índice.
    /// Sustituye la anterior si la había. Devuelve la entrada actualizada.
    pub fn guardar_miniatura(
        &self,
        id: &str,
        png: &[u8],
        texturas: &str,
    ) -> Result<Entrada, ErrorApp> {
        validar_clave_texturas(texturas)?;
        validar_png(png)?;
        if !id_valido(id) {
            return Err(ErrorApp::NoEncontrado);
        }
        existe(&self.conexion(), id)?;

        let destino = ruta_miniatura(&self.dir, id);
        let tmp = self.dir.join(format!("{id}.miniatura.png.tmp"));
        if let Err(e) = fs::write(&tmp, png) {
            let _ = fs::remove_file(&tmp);
            return Err(ErrorApp::io("guardar la miniatura", e));
        }
        if let Err(e) = reemplazar(&tmp, &destino) {
            let _ = fs::remove_file(&tmp);
            return Err(ErrorApp::io("guardar la miniatura", e));
        }
        let cambiadas = self.conexion().execute(
            "UPDATE schematics SET miniatura_fecha = ?1, miniatura_texturas = ?2 WHERE id = ?3",
            params![ahora_ms(), texturas, id],
        )?;
        if cambiadas == 0 {
            // Se eliminó mientras se escribía: el PNG recién creado sobra (es
            // un derivado que acaba de generarse, no un archivo del usuario).
            let _ = fs::remove_file(&destino);
            return Err(ErrorApp::NoEncontrado);
        }
        Ok(self.entrada_y_ruta(id)?.0)
    }

    /// Marca todas las miniaturas como obsoletas (las texturas han cambiado).
    /// Los PNG se quedan: se siguen enseñando hasta que se regeneran.
    /// Devuelve cuántas había.
    pub fn invalidar_miniaturas(&self) -> Result<usize, ErrorApp> {
        Ok(self.conexion().execute(
            "UPDATE schematics SET miniatura_texturas = NULL WHERE miniatura_texturas IS NOT NULL",
            [],
        )?)
    }
}
