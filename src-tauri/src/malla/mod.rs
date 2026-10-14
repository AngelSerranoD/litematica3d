// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Malla 3D del visor: combina las regiones ([`Mundo`]), clasifica cada block
//! state ([`formas`]), malla por chunks de 16³ ([`mallado`]) y lo empaqueta en
//! un binario ([`binario`]). El contrato con el frontend está en
//! `docs/visor.md`.

pub mod apariencia;
pub mod binario;
pub mod formas;
pub mod mallado;
pub mod mundo;

#[cfg(test)]
mod pruebas;

use std::sync::{Arc, Mutex};

pub use apariencia::{
    Cara, DatosAtlas, FuenteTexturas, Material, TexturaCara, Tipo, color_de, textura_de,
};
pub use mallado::{ChunkMalla, Malla, Opciones, mallar, mallar_con_texturas};
pub use mundo::{BloqueSeleccionado, Mundo};

/// Último schematic abierto en el visor, ya combinado, para no releer ni
/// reparsear el archivo al seleccionar bloques o al volver a mallar.
#[derive(Debug, Default)]
pub struct CacheVisor {
    ultimo: Mutex<Option<(String, Arc<Mundo>)>>,
}

impl CacheVisor {
    /// Mundo del id si es el que está en caché.
    pub fn obtener(&self, id: &str) -> Option<Arc<Mundo>> {
        let g = self.ultimo.lock().unwrap_or_else(|e| e.into_inner());
        g.as_ref()
            .filter(|(i, _)| i == id)
            .map(|(_, m)| Arc::clone(m))
    }

    /// Sustituye el contenido de la caché (solo se guarda uno).
    pub fn guardar(&self, id: &str, mundo: Arc<Mundo>) {
        let mut g = self.ultimo.lock().unwrap_or_else(|e| e.into_inner());
        *g = Some((id.to_string(), mundo));
    }

    /// Olvida un id (al borrarlo de la biblioteca).
    pub fn olvidar(&self, id: &str) {
        let mut g = self.ultimo.lock().unwrap_or_else(|e| e.into_inner());
        if g.as_ref().is_some_and(|(i, _)| i == id) {
            *g = None;
        }
    }
}
