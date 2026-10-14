// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Modelo de cambios de un schematic y su aplicación en memoria. Guardar el
//! resultado, el historial y deshacer están en `biblioteca::historial`; el
//! contrato con el frontend, en `docs/edicion.md`.
//!
//! Los block states se escriben como el texto canónico del resto de la app
//! (`minecraft:oak_stairs[facing=north,half=bottom]`, el mismo que devuelve
//! `bloque_en`); se normalizan (namespace y orden de propiedades) antes de usarlos.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::ErrorApp;
use crate::schematic::{Region, Schematic, normalizar_estado};

/// Máximo de cambios puntuales en una sola llamada.
pub const MAX_CAMBIOS: usize = 1_000_000;
/// Máximo de reemplazos en una sola llamada.
pub const MAX_REEMPLAZOS: usize = 10_000;
/// Longitud máxima de un block state, en bytes.
pub const MAX_LONGITUD_ESTADO: usize = 512;

/// Cambia el bloque de una posición.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cambio {
    /// Nombre de la región (el de `obtener` o el de `bloque_en`). Si hay
    /// varias con el mismo nombre, la última que contenga la posición.
    pub region: String,
    /// Posición en coordenadas del schematic: las de `Region.origen` y las de
    /// `BloqueSeleccionado.posicion` (no las del modelo del visor).
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// Block state nuevo; `minecraft:air` para quitar el bloque.
    pub nuevo_estado: String,
}

/// Cambia todas las apariciones de un block state, en todas las regiones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reemplazo {
    /// Block state exacto (con sus propiedades) que se sustituye.
    pub estado_origen: String,
    pub estado_destino: String,
}

/// Valida y normaliza un block state: `ns:ruta[clave=valor,…]`, con el
/// namespace y la ruta como los identificadores de Minecraft (minúsculas,
/// cifras, `_`, `-`, `.` y `/` en la ruta) y propiedades sin repetir.
pub fn validar_estado(texto: &str) -> Result<String, ErrorApp> {
    let invalido = |motivo: &str| {
        let mostrado: String = texto.trim().chars().take(80).collect();
        ErrorApp::EdicionInvalida(format!(
            "El bloque «{mostrado}» no es un block state válido: {motivo}."
        ))
    };
    let texto = texto.trim();
    if texto.is_empty() {
        return Err(invalido("está vacío"));
    }
    if texto.len() > MAX_LONGITUD_ESTADO {
        return Err(invalido(&format!(
            "pasa de {MAX_LONGITUD_ESTADO} caracteres"
        )));
    }
    if texto.matches('[').count() > 1 || texto.matches(']').count() > 1 {
        return Err(invalido("tiene corchetes de más"));
    }
    if let Some((_, resto)) = texto.split_once('[')
        && (!resto.ends_with(']') || resto.len() < 2)
    {
        return Err(invalido("las propiedades van entre corchetes"));
    }
    let estado = normalizar_estado(texto);
    let (nombre, props) = match estado.split_once('[') {
        None => (estado.as_str(), None),
        Some((n, resto)) => (n, Some(resto.strip_suffix(']').unwrap_or(resto))),
    };
    let Some((ns, ruta)) = nombre.split_once(':') else {
        return Err(invalido("falta el nombre"));
    };
    let ok_ns = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.".contains(c);
    if ns.is_empty() || !ns.chars().all(ok_ns) {
        return Err(invalido("el namespace no es válido"));
    }
    if ruta.is_empty() || !ruta.chars().all(|c| ok_ns(c) || c == '/') {
        return Err(invalido("el nombre no es válido"));
    }
    if let Some(props) = props {
        let ok_prop = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c))
        };
        let mut anterior: Option<&str> = None;
        for par in props.split(',') {
            let (k, v) = par.split_once('=').unwrap_or((par, ""));
            if !ok_prop(k) || !ok_prop(v) {
                return Err(invalido("tiene una propiedad mal escrita"));
            }
            // Vienen ordenadas por clave: una repetida queda al lado.
            if anterior == Some(k) {
                return Err(invalido("repite una propiedad"));
            }
            anterior = Some(k);
        }
    }
    Ok(estado)
}

/// Cambio puntual ya validado: región e índice lineal dentro de ella.
struct CambioResuelto {
    region: usize,
    indice: usize,
    estado: String,
}

/// Aplica primero los reemplazos (en orden: uno puede encadenar con el
/// siguiente) y después los cambios puntuales (en orden: si dos tocan la
/// misma posición, gana el último), así que un cambio puntual manda sobre un
/// reemplazo en su posición. Las paletas se recompactan (aire en el 0, sin
/// repetidos ni entradas sin usar) y, donde cambia el tipo de bloque, se
/// quitan la entidad de bloque y los ticks pendientes que hubiera (el cofre
/// que pasa a ser piedra pierde su contenido).
///
/// Todo se valida antes de tocar nada: si algo falla, `s` queda igual.
/// Devuelve cuántas posiciones han cambiado de verdad.
pub fn aplicar(
    s: &mut Schematic,
    cambios: &[Cambio],
    reemplazos: &[Reemplazo],
) -> Result<u64, ErrorApp> {
    if cambios.len() > MAX_CAMBIOS {
        return Err(ErrorApp::EdicionInvalida(format!(
            "Son demasiados cambios de una vez (el máximo es {MAX_CAMBIOS})."
        )));
    }
    if reemplazos.len() > MAX_REEMPLAZOS {
        return Err(ErrorApp::EdicionInvalida(format!(
            "Son demasiados reemplazos de una vez (el máximo es {MAX_REEMPLAZOS})."
        )));
    }

    // 1. Validación completa, sin tocar el schematic.
    let mut pares = Vec::with_capacity(reemplazos.len());
    for r in reemplazos {
        let origen = validar_estado(&r.estado_origen)?;
        let destino = validar_estado(&r.estado_destino)?;
        if origen != destino {
            pares.push((origen, destino));
        }
    }
    let mut resueltos = Vec::with_capacity(cambios.len());
    for c in cambios {
        let estado = validar_estado(&c.nuevo_estado)?;
        let (region, indice) = resolver_posicion(s, c)?;
        resueltos.push(CambioResuelto {
            region,
            indice,
            estado,
        });
    }

    // 2. Aplicación región por región.
    let mut por_region: Vec<Vec<&CambioResuelto>> = vec![Vec::new(); s.regiones.len()];
    for c in &resueltos {
        por_region[c.region].push(c);
    }
    let mut total = 0u64;
    for (r, cambios) in s.regiones.iter_mut().zip(por_region) {
        total += aplicar_en_region(r, &pares, &cambios);
    }
    Ok(total)
}

/// Región (índice) e índice lineal de la posición de un cambio.
fn resolver_posicion(s: &Schematic, c: &Cambio) -> Result<(usize, usize), ErrorApp> {
    let mut alguna = false;
    for (i, r) in s.regiones.iter().enumerate().rev() {
        if r.nombre != c.region {
            continue;
        }
        alguna = true;
        let rel = [c.x, c.y, c.z]
            .iter()
            .zip(r.origen)
            .map(|(&p, o)| i64::from(p) - i64::from(o))
            .collect::<Vec<_>>();
        let dentro = rel.iter().all(|&v| (0..=i64::from(u32::MAX)).contains(&v));
        if dentro
            && let Some(indice) = r.indice(rel[0] as u32, rel[1] as u32, rel[2] as u32)
            && indice < r.bloques.len()
        {
            return Ok((i, indice));
        }
    }
    if !alguna {
        return Err(ErrorApp::RegionNoEncontrada {
            nombre: c.region.clone(),
        });
    }
    Err(ErrorApp::EdicionInvalida(format!(
        "La posición ({}, {}, {}) está fuera de la región «{}».",
        c.x, c.y, c.z, c.region
    )))
}

/// Nombre sin propiedades de un block state.
fn tipo_de(estado: &str) -> &str {
    estado.split('[').next().unwrap_or(estado)
}

fn aplicar_en_region(
    r: &mut Region,
    pares: &[(String, String)],
    cambios: &[&CambioResuelto],
) -> u64 {
    if pares.is_empty() && cambios.is_empty() {
        return 0;
    }
    let paleta_original = r.paleta.clone();
    // Tipo de bloque, antes de cambiar nada, en cada posición con entidad de
    // bloque o tick pendiente.
    let antes_entidades = tipos_en_lista(r, r.extras.entidades_bloque.as_ref());
    let antes_ticks = tipos_en_lista(r, r.extras.ticks_bloque.as_ref());

    // Reemplazos: cambian el texto de las entradas de la paleta (los índices
    // de `bloques` no se tocan, así que siguen apuntando al estado original).
    let mut cambia = vec![false; r.paleta.len()];
    if !pares.is_empty() {
        for (i, estado) in r.paleta.iter_mut().enumerate() {
            let inicial = estado.clone();
            for (origen, destino) in pares {
                if estado == origen {
                    *estado = destino.clone();
                }
            }
            cambia[i] = *estado != inicial;
        }
    }
    let mut total = if cambia.iter().any(|&c| c) {
        r.bloques
            .iter()
            .filter(|&&p| cambia.get(p as usize).copied().unwrap_or(false))
            .count() as u64
    } else {
        0
    };

    // Cambios puntuales: se apunta el índice original de cada posición tocada
    // para contar al final solo las que acaban distintas.
    if !cambios.is_empty() {
        let mut indices: HashMap<String, u32> = HashMap::new();
        for (i, e) in r.paleta.iter().enumerate() {
            indices.entry(e.clone()).or_insert(i as u32);
        }
        let mut originales: HashMap<usize, u32> = HashMap::new();
        for c in cambios {
            let p = match indices.get(&c.estado) {
                Some(&p) => p,
                None => {
                    r.paleta.push(c.estado.clone());
                    let p = (r.paleta.len() - 1) as u32;
                    indices.insert(c.estado.clone(), p);
                    p
                }
            };
            originales.entry(c.indice).or_insert(r.bloques[c.indice]);
            r.bloques[c.indice] = p;
        }
        for (indice, original) in originales {
            let o = original as usize;
            if cambia.get(o).copied().unwrap_or(false) {
                total -= 1;
            }
            let final_ = &r.paleta[r.bloques[indice] as usize];
            if paleta_original.get(o) != Some(final_) {
                total += 1;
            }
        }
    }

    recompactar(r);
    filtrar_lista(r, ExtraConPosicion::EntidadesBloque, &antes_entidades);
    filtrar_lista(r, ExtraConPosicion::TicksBloque, &antes_ticks);
    total
}

/// Deja la paleta con `minecraft:air` en el 0 y después solo los estados que
/// se usan, sin repetir, en orden de aparición en la paleta anterior.
fn recompactar(r: &mut Region) {
    let mut usado = vec![false; r.paleta.len()];
    for &p in &r.bloques {
        if let Some(u) = usado.get_mut(p as usize) {
            *u = true;
        }
    }
    let mut nueva = vec!["minecraft:air".to_string()];
    let mut indices: HashMap<&str, u32> = HashMap::from([("minecraft:air", 0)]);
    let mut remapeo = vec![0u32; r.paleta.len()];
    for (i, estado) in r.paleta.iter().enumerate() {
        if !usado[i] {
            continue;
        }
        remapeo[i] = *indices.entry(estado.as_str()).or_insert_with(|| {
            nueva.push(estado.clone());
            (nueva.len() - 1) as u32
        });
    }
    for p in r.bloques.iter_mut() {
        *p = remapeo.get(*p as usize).copied().unwrap_or(0);
    }
    r.paleta = nueva;
}

#[derive(Clone, Copy)]
enum ExtraConPosicion {
    EntidadesBloque,
    TicksBloque,
}

/// Posición (`x`, `y`, `z` enteros relativos a la esquina mínima) de un
/// elemento de `TileEntities` o `PendingBlockTicks`.
fn posicion_nbt(v: &fastnbt::Value) -> Option<[u32; 3]> {
    let fastnbt::Value::Compound(m) = v else {
        return None;
    };
    let eje = |k: &str| match m.get(k)? {
        fastnbt::Value::Int(i) => u32::try_from(*i).ok(),
        fastnbt::Value::Short(i) => u32::try_from(*i).ok(),
        fastnbt::Value::Byte(i) => u32::try_from(*i).ok(),
        _ => None,
    };
    Some([eje("x")?, eje("y")?, eje("z")?])
}

/// Tipo de bloque (sin propiedades) en la posición de cada elemento de la
/// lista; `None` si el elemento no tiene posición legible.
fn tipos_en_lista(r: &Region, lista: Option<&fastnbt::Value>) -> Vec<Option<String>> {
    let Some(fastnbt::Value::List(elementos)) = lista else {
        return Vec::new();
    };
    elementos
        .iter()
        .map(|e| {
            let [x, y, z] = posicion_nbt(e)?;
            r.bloque(x, y, z).map(|b| tipo_de(b).to_string())
        })
        .collect()
}

/// Quita los elementos cuya posición ha cambiado de tipo de bloque.
fn filtrar_lista(r: &mut Region, cual: ExtraConPosicion, antes: &[Option<String>]) {
    if antes.iter().all(Option::is_none) {
        return;
    }
    let ahora: Vec<Option<String>> = {
        let lista = match cual {
            ExtraConPosicion::EntidadesBloque => r.extras.entidades_bloque.as_ref(),
            ExtraConPosicion::TicksBloque => r.extras.ticks_bloque.as_ref(),
        };
        tipos_en_lista(r, lista)
    };
    let lista = match cual {
        ExtraConPosicion::EntidadesBloque => r.extras.entidades_bloque.as_mut(),
        ExtraConPosicion::TicksBloque => r.extras.ticks_bloque.as_mut(),
    };
    let Some(fastnbt::Value::List(elementos)) = lista else {
        return;
    };
    let mut i = 0;
    elementos.retain(|_| {
        let quedar = match (antes.get(i), ahora.get(i)) {
            (Some(Some(a)), Some(Some(b))) => a == b,
            _ => true,
        };
        i += 1;
        quedar
    });
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::schematic::{Dimensiones, ExtrasRegion, Formato, modelo::caja_englobante};

    /// Región 3×2×2 en `origen` con piedra en todo salvo aire en (0,0,0).
    fn region(nombre: &str, origen: [i32; 3]) -> Region {
        let tamano = Dimensiones {
            ancho: 3,
            alto: 2,
            largo: 2,
        };
        let mut bloques = vec![1u32; tamano.volumen() as usize];
        bloques[0] = 0;
        Region {
            nombre: nombre.into(),
            origen,
            tamano,
            paleta: vec!["minecraft:air".into(), "minecraft:stone".into()],
            bloques,
            extras: ExtrasRegion::default(),
        }
    }

    fn schem(regiones: Vec<Region>) -> Schematic {
        Schematic {
            formato: Formato::Litematic,
            nombre: None,
            autor: None,
            descripcion: None,
            version_datos: None,
            fecha_creacion: None,
            dimensiones: caja_englobante(&regiones),
            regiones,
        }
    }

    fn cambio(region: &str, p: [i32; 3], estado: &str) -> Cambio {
        Cambio {
            region: region.into(),
            x: p[0],
            y: p[1],
            z: p[2],
            nuevo_estado: estado.into(),
        }
    }

    fn reemplazo(origen: &str, destino: &str) -> Reemplazo {
        Reemplazo {
            estado_origen: origen.into(),
            estado_destino: destino.into(),
        }
    }

    #[test]
    fn valida_y_normaliza_estados() {
        assert_eq!(validar_estado(" stone ").unwrap(), "minecraft:stone");
        assert_eq!(
            validar_estado("oak_stairs[half=bottom,facing=north]").unwrap(),
            "minecraft:oak_stairs[facing=north,half=bottom]"
        );
        assert_eq!(validar_estado("mod:a/b.c").unwrap(), "mod:a/b.c");
        for malo in [
            "",
            "   ",
            "Stone",
            "minecraft:",
            ":stone",
            "a:b:c",
            "stone[",
            "stone[a]",
            "stone[a=]",
            "stone[a=1,a=2]",
            "stone[a=1]]",
            "stone[[a=1]",
            "piedra con espacios",
            "minecraft:stone\"",
        ] {
            let e = validar_estado(malo).unwrap_err();
            assert_eq!(e.tipo(), "edicionInvalida", "{malo:?}");
        }
        let largo = format!("minecraft:{}", "a".repeat(MAX_LONGITUD_ESTADO));
        assert!(validar_estado(&largo).is_err());
    }

    #[test]
    fn cambio_puntual_en_coordenadas_del_schematic() {
        let mut s = schem(vec![region("Casa", [10, 64, -5])]);
        let n = aplicar(
            &mut s,
            &[
                cambio("Casa", [11, 64, -5], "oak_planks"),
                // Mismo estado que ya tenía: no cuenta.
                cambio("Casa", [12, 64, -5], "minecraft:stone"),
                // Quitar un bloque.
                cambio("Casa", [10, 65, -4], "minecraft:air"),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(n, 2);
        let r = &s.regiones[0];
        assert_eq!(r.bloque(1, 0, 0), Some("minecraft:oak_planks"));
        assert_eq!(r.bloque(2, 0, 0), Some("minecraft:stone"));
        assert_eq!(r.bloque(0, 1, 1), Some("minecraft:air"));
        assert_eq!(r.paleta[0], "minecraft:air");
        assert_eq!(s.total_bloques(), 12 - 1 - 1);
    }

    #[test]
    fn reemplazo_y_paleta_recompactada() {
        let mut s = schem(vec![region("A", [0, 0, 0]), region("B", [5, 0, 0])]);
        let n = aplicar(&mut s, &[], &[reemplazo("stone", "minecraft:granite")]).unwrap();
        assert_eq!(n, 22);
        for r in &s.regiones {
            // La piedra ya no está ni en la paleta.
            assert_eq!(r.paleta, ["minecraft:air", "minecraft:granite"]);
            assert_eq!(r.bloque(1, 0, 0), Some("minecraft:granite"));
        }
        // Reemplazar a aire deja la paleta solo con aire.
        let n = aplicar(&mut s, &[], &[reemplazo("granite", "air")]).unwrap();
        assert_eq!(n, 22);
        assert_eq!(s.total_bloques(), 0);
        assert_eq!(s.regiones[0].paleta, ["minecraft:air"]);
    }

    #[test]
    fn el_cambio_puntual_manda_sobre_el_reemplazo() {
        let mut s = schem(vec![region("A", [0, 0, 0])]);
        let n = aplicar(
            &mut s,
            &[
                cambio("A", [1, 0, 0], "minecraft:dirt"),
                // Vuelve a dejar piedra: esa posición no cuenta como cambiada.
                cambio("A", [2, 0, 0], "minecraft:stone"),
            ],
            &[reemplazo("stone", "granite")],
        )
        .unwrap();
        let r = &s.regiones[0];
        assert_eq!(r.bloque(1, 0, 0), Some("minecraft:dirt"));
        assert_eq!(r.bloque(2, 0, 0), Some("minecraft:stone"));
        assert_eq!(r.bloque(0, 1, 0), Some("minecraft:granite"));
        assert_eq!(n, 10);
    }

    #[test]
    fn errores_claros_y_sin_tocar_nada() {
        let mut s = schem(vec![region("A", [0, 0, 0])]);
        let antes = s.regiones[0].bloques.clone();
        let e = aplicar(&mut s, &[cambio("Z", [0, 0, 0], "stone")], &[]).unwrap_err();
        assert_eq!(e.tipo(), "regionNoEncontrada");
        for fuera in [[3, 0, 0], [-1, 0, 0], [0, 2, 0], [0, 0, i32::MIN]] {
            let e = aplicar(
                &mut s,
                &[cambio("A", [1, 0, 0], "dirt"), cambio("A", fuera, "dirt")],
                &[],
            )
            .unwrap_err();
            assert_eq!(e.tipo(), "edicionInvalida", "{fuera:?}");
        }
        let e = aplicar(
            &mut s,
            &[cambio("A", [1, 0, 0], "dirt")],
            &[reemplazo("stone", "No Vale")],
        )
        .unwrap_err();
        assert_eq!(e.tipo(), "edicionInvalida");
        assert_eq!(s.regiones[0].bloques, antes);
        assert_eq!(s.regiones[0].paleta.len(), 2);
    }

    #[test]
    fn regiones_con_el_mismo_nombre() {
        let mut s = schem(vec![region("R", [0, 0, 0]), region("R", [10, 0, 0])]);
        aplicar(&mut s, &[cambio("R", [11, 0, 0], "dirt")], &[]).unwrap();
        assert_eq!(s.regiones[1].bloque(1, 0, 0), Some("minecraft:dirt"));
        aplicar(&mut s, &[cambio("R", [1, 0, 0], "dirt")], &[]).unwrap();
        assert_eq!(s.regiones[0].bloque(1, 0, 0), Some("minecraft:dirt"));
    }

    #[test]
    fn quita_la_entidad_de_bloque_si_cambia_el_tipo() {
        use fastnbt::Value;
        let entidad = |x: i32, id: &str| {
            Value::Compound(HashMap::from([
                ("x".to_string(), Value::Int(x)),
                ("y".to_string(), Value::Int(0)),
                ("z".to_string(), Value::Int(0)),
                ("id".to_string(), Value::String(id.into())),
            ]))
        };
        let mut r = region("A", [0, 0, 0]);
        r.paleta.push("minecraft:chest[facing=north]".into());
        r.bloques[1] = 2;
        r.bloques[2] = 2;
        r.extras.entidades_bloque = Some(Value::List(vec![
            entidad(1, "minecraft:chest"),
            entidad(2, "minecraft:chest"),
        ]));
        let mut s = schem(vec![r]);
        aplicar(
            &mut s,
            &[
                // Solo gira: el cofre sigue siendo cofre y conserva su contenido.
                cambio("A", [1, 0, 0], "chest[facing=south]"),
                cambio("A", [2, 0, 0], "stone"),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(
            s.regiones[0].extras.entidades_bloque,
            Some(Value::List(vec![entidad(1, "minecraft:chest")]))
        );
    }
}
