// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas de la edición con historial (docs/edicion.md): aplicar cambios,
//! historial, restaurar y rehacer, exportar, límite de versiones con la
//! Papelera simulada y atomicidad ante fallos.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use schematic_lab_lib::biblioteca::{
    Biblioteca, Entrada, Importacion, MAX_VERSIONES, Motivo, Papelera, sha256_hex,
};
use schematic_lab_lib::edicion::{Cambio, Reemplazo};
use schematic_lab_lib::schematic::{self, Formato, Schematic, es_aire};

fn dato(nombre: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("datos")
        .join(nombre)
}

/// Papelera de mentira: mueve lo que recibe a una carpeta del directorio
/// temporal y apunta qué ha recibido. Se puede hacer fallar.
#[derive(Clone)]
struct PapeleraFalsa {
    dir: PathBuf,
    recibidos: Arc<Mutex<Vec<PathBuf>>>,
    fallar: Arc<AtomicBool>,
}

impl PapeleraFalsa {
    fn nueva(dir: &Path) -> Self {
        let dir = dir.join("papelera");
        fs::create_dir_all(&dir).unwrap();
        PapeleraFalsa {
            dir,
            recibidos: Arc::default(),
            fallar: Arc::default(),
        }
    }

    fn recibidos(&self) -> Vec<PathBuf> {
        self.recibidos.lock().unwrap().clone()
    }
}

impl Papelera for PapeleraFalsa {
    fn enviar(&self, rutas: &[PathBuf]) -> Result<(), String> {
        if self.fallar.load(Ordering::SeqCst) {
            return Err("Papelera simulada sin servicio".into());
        }
        let mut recibidos = self.recibidos.lock().unwrap();
        for r in rutas {
            let destino = self.dir.join(format!(
                "{}-{}",
                recibidos.len(),
                r.file_name().unwrap().to_string_lossy()
            ));
            fs::rename(r, destino).map_err(|e| e.to_string())?;
            recibidos.push(r.clone());
        }
        Ok(())
    }
}

/// Biblioteca en `tmp/datos` con la Papelera simulada en `tmp/papelera`.
fn abrir(tmp: &Path) -> (Biblioteca, PapeleraFalsa) {
    let papelera = PapeleraFalsa::nueva(tmp);
    let b = Biblioteca::abrir_con_papelera(&tmp.join("datos"), Box::new(papelera.clone())).unwrap();
    (b, papelera)
}

fn importar(b: &Biblioteca, nombre: &str) -> Entrada {
    match b.importar(&dato(nombre)).unwrap() {
        Importacion::Importado(e) => e,
        Importacion::Duplicado(e) => panic!("no debía ser duplicado: {}", e.nombre),
    }
}

/// Archivos (y carpetas) de primer nivel en la carpeta de la biblioteca.
fn copias(b: &Biblioteca) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(b.carpeta_archivos())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// Archivos de la carpeta de historial de un id.
fn en_historial(b: &Biblioteca, id: &str) -> Vec<String> {
    let dir = b.carpeta_archivos().join("historial").join(id);
    let Ok(lectura) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<String> = lectura
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn ruta_copia(b: &Biblioteca, e: &Entrada) -> PathBuf {
    b.carpeta_archivos()
        .join(format!("{}.{}", e.id, e.formato.extension_habitual()))
}

/// Primer bloque que no es aire: región, posición (coordenadas del
/// schematic) y estado.
fn primer_bloque(s: &Schematic) -> (String, [i32; 3], String) {
    for r in &s.regiones {
        for y in 0..r.tamano.alto {
            for z in 0..r.tamano.largo {
                for x in 0..r.tamano.ancho {
                    let b = r.bloque(x, y, z).unwrap();
                    if !es_aire(b) {
                        let p = [
                            r.origen[0] + x as i32,
                            r.origen[1] + y as i32,
                            r.origen[2] + z as i32,
                        ];
                        return (r.nombre.clone(), p, b.to_string());
                    }
                }
            }
        }
    }
    panic!("schematic sin bloques");
}

/// Estado en una posición del schematic (coordenadas del schematic).
fn estado_en(s: &Schematic, region: &str, p: [i32; 3]) -> String {
    let r = s
        .regiones
        .iter()
        .rev()
        .find(|r| r.nombre == region)
        .unwrap();
    let rel: Vec<u32> = (0..3).map(|e| (p[e] - r.origen[e]) as u32).collect();
    r.bloque(rel[0], rel[1], rel[2]).unwrap().to_string()
}

/// Veces que aparece un estado, en todas las regiones.
fn cuantos(s: &Schematic, estado: &str) -> u64 {
    s.regiones
        .iter()
        .map(|r| {
            r.bloques
                .iter()
                .filter(|&&p| r.paleta[p as usize] == estado)
                .count() as u64
        })
        .sum()
}

/// Estado más repetido que no es aire.
fn mas_comun(s: &Schematic) -> String {
    let mut cuenta = std::collections::HashMap::<&str, u64>::new();
    for r in &s.regiones {
        for &p in &r.bloques {
            let e = r.paleta[p as usize].as_str();
            if !es_aire(e) {
                *cuenta.entry(e).or_default() += 1;
            }
        }
    }
    cuenta
        .into_iter()
        .max_by_key(|&(e, n)| (n, std::cmp::Reverse(e)))
        .unwrap()
        .0
        .to_string()
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

fn hash_en_indice(datos: &Path, id: &str) -> String {
    let conn = rusqlite::Connection::open(datos.join("biblioteca.sqlite3")).unwrap();
    conn.query_row("SELECT hash FROM schematics WHERE id = ?1", [id], |f| {
        f.get(0)
    })
    .unwrap()
}

/// PNG cuadrado y transparente para la miniatura.
fn png(lado: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, lado, lado);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&vec![0; (lado * lado * 4) as usize])
            .unwrap();
    }
    out
}

#[test]
fn aplicar_un_cambio_puntual() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, _) = abrir(tmp.path());
    let e = importar(&b, "litematic_simple.litematic");
    b.guardar_miniatura(&e.id, &png(64), "r1|-").unwrap();
    let original = fs::read(ruta_copia(&b, &e)).unwrap();
    let (_, antes) = b.cargar(&e.id).unwrap();
    let (region, p, estado) = primer_bloque(&antes);

    let r = b
        .aplicar_cambios(&e.id, &[cambio(&region, p, "diamond_block")], &[])
        .unwrap();
    assert_eq!(r.cambios, 1);
    let version = r.version.unwrap();
    let nueva = &r.entrada;
    assert_eq!(nueva.formato, Formato::Litematic);
    assert_eq!(nueva.total_bloques, e.total_bloques);
    assert_eq!(nueva.nombre, e.nombre);
    // La miniatura sigue ahí pero obsoleta.
    let m = nueva.miniatura.clone().unwrap();
    assert_eq!(m.texturas, None);
    assert!(Path::new(&m.ruta).exists());

    // La copia es la nueva; el índice la describe.
    let bytes = fs::read(ruta_copia(&b, nueva)).unwrap();
    assert_ne!(bytes, original);
    assert_eq!(nueva.tamano_archivo, bytes.len() as u64);
    assert_eq!(
        hash_en_indice(&tmp.path().join("datos"), &e.id),
        sha256_hex(&bytes)
    );
    let (_, despues) = b.cargar(&e.id).unwrap();
    assert_eq!(estado_en(&despues, &region, p), "minecraft:diamond_block");
    assert_ne!(estado, "minecraft:diamond_block");
    // Conserva los metadatos del original.
    assert_eq!(despues.autor, antes.autor);
    assert_eq!(despues.version_datos, antes.version_datos);

    // La versión anterior está en el historial, idéntica al original.
    let h = b.historial(&e.id).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].version, version);
    assert_eq!(h[0].motivo, Motivo::Aplicar);
    assert_eq!(h[0].cambios, 1);
    assert_eq!(h[0].formato, Formato::Litematic);
    assert_eq!(h[0].total_bloques, e.total_bloques);
    assert_eq!(h[0].tamano_archivo, original.len() as u64);
    assert_eq!(en_historial(&b, &e.id), [format!("{version}.litematic")]);
    let guardada = b
        .carpeta_archivos()
        .join("historial")
        .join(&e.id)
        .join(format!("{version}.litematic"));
    assert_eq!(fs::read(guardada).unwrap(), original);
    // Ni temporales ni restos.
    assert_eq!(
        copias(&b),
        [
            format!("{}.litematic", e.id),
            format!("{}.miniatura.png", e.id),
            "historial".into()
        ]
    );

    // Deshacer = restaurar esa versión.
    let r = b.restaurar_version(&e.id, version).unwrap();
    assert_eq!(r.cambios, 0);
    assert_eq!(fs::read(ruta_copia(&b, &r.entrada)).unwrap(), original);
    assert_eq!(
        hash_en_indice(&tmp.path().join("datos"), &e.id),
        sha256_hex(&original)
    );
}

#[test]
fn reemplazo_restaurar_y_rehacer_con_cambio_de_formato() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, papelera) = abrir(tmp.path());
    let e = importar(&b, "sponge_v2.schem");
    assert_eq!(e.formato, Formato::Sponge);
    let original = fs::read(ruta_copia(&b, &e)).unwrap();
    let (_, antes) = b.cargar(&e.id).unwrap();
    let comun = mas_comun(&antes);
    let n = cuantos(&antes, &comun);

    let r = b
        .aplicar_cambios(
            &e.id,
            &[],
            &[Reemplazo {
                estado_origen: comun.clone(),
                estado_destino: "minecraft:gold_block".into(),
            }],
        )
        .unwrap();
    assert_eq!(r.cambios, n);
    let v_original = r.version.unwrap();
    // Se guarda como .litematic; el .schem pasa entero al historial.
    assert_eq!(r.entrada.formato, Formato::Litematic);
    assert_eq!(
        copias(&b),
        [format!("{}.litematic", e.id), "historial".into()]
    );
    assert_eq!(en_historial(&b, &e.id), [format!("{v_original}.schem")]);
    let (_, editado) = b.cargar(&e.id).unwrap();
    assert_eq!(editado.formato, Formato::Litematic);
    assert_eq!(cuantos(&editado, &comun), 0);
    assert_eq!(cuantos(&editado, "minecraft:gold_block"), n);
    assert_eq!(editado.total_bloques(), antes.total_bloques());
    // Sin nombre en el .schem: el .litematic lleva el de la biblioteca.
    assert_eq!(editado.nombre.as_deref(), Some(e.nombre.as_str()));
    assert_eq!(b.historial(&e.id).unwrap()[0].formato, Formato::Sponge);

    // Restaurar el original: vuelve a ser Sponge y la editada va al historial.
    let r = b.restaurar_version(&e.id, v_original).unwrap();
    assert_eq!(r.entrada.formato, Formato::Sponge);
    assert_eq!(r.entrada.tamano_archivo, original.len() as u64);
    assert_eq!(fs::read(ruta_copia(&b, &r.entrada)).unwrap(), original);
    let v_editada = r.version.unwrap();
    assert!(v_editada > v_original);
    let h = b.historial(&e.id).unwrap();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].version, v_editada);
    assert_eq!(h[0].motivo, Motivo::Restaurar);
    assert_eq!(h[0].formato, Formato::Litematic);
    assert_eq!(h[1].version, v_original);
    assert_eq!(copias(&b), [format!("{}.schem", e.id), "historial".into()]);

    // Rehacer: restaurar la editada.
    let r = b.restaurar_version(&e.id, v_editada).unwrap();
    assert_eq!(r.entrada.formato, Formato::Litematic);
    let (_, rehecho) = b.cargar(&e.id).unwrap();
    assert_eq!(cuantos(&rehecho, "minecraft:gold_block"), n);
    assert_eq!(b.historial(&e.id).unwrap().len(), 3);
    assert_eq!(
        copias(&b),
        [format!("{}.litematic", e.id), "historial".into()]
    );
    // Nada ha pasado por la Papelera.
    assert!(papelera.recibidos().is_empty());

    // Versión que no existe.
    assert_eq!(
        b.restaurar_version(&e.id, 1).unwrap_err().tipo(),
        "versionNoEncontrada"
    );
}

#[test]
fn sin_cambios_reales_no_se_toca_nada() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, _) = abrir(tmp.path());
    let e = importar(&b, "litematic_simple.litematic");
    let (_, s) = b.cargar(&e.id).unwrap();
    let (region, p, estado) = primer_bloque(&s);
    let r = b
        .aplicar_cambios(&e.id, &[cambio(&region, p, &estado)], &[])
        .unwrap();
    assert_eq!(r.cambios, 0);
    assert_eq!(r.version, None);
    assert_eq!(r.entrada, e);
    let r = b.aplicar_cambios(&e.id, &[], &[]).unwrap();
    assert_eq!(r.version, None);
    assert!(b.historial(&e.id).unwrap().is_empty());
    assert_eq!(copias(&b), [format!("{}.litematic", e.id)]);
}

#[test]
fn cambios_invalidos_no_dejan_rastro() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, _) = abrir(tmp.path());
    let e = importar(&b, "litematic_simple.litematic");
    let original = fs::read(ruta_copia(&b, &e)).unwrap();
    let (_, s) = b.cargar(&e.id).unwrap();
    let (region, p, _) = primer_bloque(&s);
    let fuera = [p[0], p[1] + 10_000, p[2]];
    for (que, cambios, tipo) in [
        (
            "región",
            vec![cambio("No existe", p, "stone")],
            "regionNoEncontrada",
        ),
        (
            "fuera",
            vec![cambio(&region, fuera, "stone")],
            "edicionInvalida",
        ),
        (
            "estado",
            vec![cambio(&region, p, "Piedra Rara")],
            "edicionInvalida",
        ),
    ] {
        let err = b.aplicar_cambios(&e.id, &cambios, &[]).unwrap_err();
        assert_eq!(err.tipo(), tipo, "{que}");
    }
    assert_eq!(
        b.aplicar_cambios("no-existe", &[], &[]).unwrap_err().tipo(),
        "noEncontrado"
    );
    assert_eq!(fs::read(ruta_copia(&b, &e)).unwrap(), original);
    assert!(b.historial(&e.id).unwrap().is_empty());
    assert_eq!(copias(&b), [format!("{}.litematic", e.id)]);
    assert_eq!(b.historial("no-existe").unwrap_err().tipo(), "noEncontrado");
}

#[test]
fn exportar_solo_a_litematic() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, _) = abrir(tmp.path());
    let lit = importar(&b, "litematic_simple.litematic");
    let mce = importar(&b, "mcedit.schematic");
    let fuera = tmp.path().join("exportados");
    fs::create_dir_all(&fuera).unwrap();

    for nombre in ["casa.schem", "casa.litematic.exe", "casa", "casa.txt"] {
        let ruta = fuera.join(nombre);
        let err = b.exportar(&lit.id, &ruta).unwrap_err();
        assert_eq!(err.tipo(), "nombreInvalido", "{nombre}");
        assert!(!ruta.exists());
    }
    let err = b
        .exportar(&lit.id, Path::new("relativa.litematic"))
        .unwrap_err();
    assert_eq!(err.tipo(), "nombreInvalido");
    assert!(!Path::new("relativa.litematic").exists());
    // Ni dentro de la carpeta de copias, donde pisaría la de otro schematic.
    let copia = ruta_copia(&b, &lit);
    let antes = fs::read(&copia).unwrap();
    for destino in [
        b.carpeta_archivos().join("pisar.litematic"),
        b.carpeta_archivos().join(format!("{}.litematic", mce.id)),
    ] {
        let err = b.exportar(&lit.id, &destino).unwrap_err();
        assert_eq!(err.tipo(), "nombreInvalido", "{}", destino.display());
    }
    assert!(!b.carpeta_archivos().join("pisar.litematic").exists());
    assert_eq!(fs::read(&copia).unwrap(), antes);
    assert_eq!(
        b.exportar("no-existe", &fuera.join("x.litematic"))
            .unwrap_err()
            .tipo(),
        "noEncontrado"
    );

    // Un .litematic sale tal cual.
    let destino = fuera.join("Casa.LITEMATIC");
    b.exportar(&lit.id, &destino).unwrap();
    assert_eq!(
        fs::read(&destino).unwrap(),
        fs::read(ruta_copia(&b, &lit)).unwrap()
    );
    // Sobrescribir uno que ya existe.
    b.exportar(&lit.id, &destino).unwrap();

    // Otro formato se convierte.
    let destino = fuera.join("mcedit.litematic");
    b.exportar(&mce.id, &destino).unwrap();
    let s = schematic::cargar(&fs::read(&destino).unwrap()).unwrap();
    assert_eq!(s.formato, Formato::Litematic);
    assert_eq!(s.total_bloques(), mce.total_bloques);
    assert_eq!(s.nombre.as_deref(), Some("mcedit"));
    // Sin temporales junto al destino; la copia de la biblioteca no cambia.
    let mut en_fuera: Vec<String> = fs::read_dir(&fuera)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    en_fuera.sort();
    assert_eq!(en_fuera, ["Casa.LITEMATIC", "mcedit.litematic"]);
    assert_eq!(b.obtener(&mce.id).unwrap().entrada.formato, Formato::Mcedit);
}

#[test]
fn limite_de_versiones_y_papelera() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, papelera) = abrir(tmp.path());
    let e = importar(&b, "litematic_simple.litematic");
    let (_, s) = b.cargar(&e.id).unwrap();
    let (region, p, _) = primer_bloque(&s);
    let estados = ["minecraft:gold_block", "minecraft:iron_block"];
    let mut versiones = Vec::new();
    let total = MAX_VERSIONES + 5;
    for i in 0..total {
        let r = b
            .aplicar_cambios(&e.id, &[cambio(&region, p, estados[i % 2])], &[])
            .unwrap();
        versiones.push(r.version.unwrap());
    }
    let h = b.historial(&e.id).unwrap();
    assert_eq!(h.len(), MAX_VERSIONES);
    // Quedan las 20 más recientes, de la más nueva a la más antigua.
    let esperadas: Vec<i64> = versiones
        .iter()
        .rev()
        .take(MAX_VERSIONES)
        .copied()
        .collect();
    assert_eq!(h.iter().map(|v| v.version).collect::<Vec<_>>(), esperadas);
    assert_eq!(en_historial(&b, &e.id).len(), MAX_VERSIONES);
    // Las 5 más antiguas han ido a la Papelera (nada más).
    let recibidos = papelera.recibidos();
    assert_eq!(recibidos.len(), 5);
    for (r, v) in recibidos.iter().zip(&versiones) {
        assert_eq!(
            r.file_name().unwrap().to_string_lossy(),
            format!("{v}.litematic")
        );
    }
    assert_eq!(
        b.restaurar_version(&e.id, versiones[0]).unwrap_err().tipo(),
        "versionNoEncontrada"
    );

    // Si la Papelera falla, la edición sale bien y se poda en la siguiente.
    papelera.fallar.store(true, Ordering::SeqCst);
    b.aplicar_cambios(&e.id, &[cambio(&region, p, "minecraft:stone")], &[])
        .unwrap();
    assert_eq!(b.historial(&e.id).unwrap().len(), MAX_VERSIONES + 1);
    papelera.fallar.store(false, Ordering::SeqCst);
    b.aplicar_cambios(&e.id, &[cambio(&region, p, "minecraft:dirt")], &[])
        .unwrap();
    assert_eq!(b.historial(&e.id).unwrap().len(), MAX_VERSIONES);
    assert_eq!(papelera.recibidos().len(), 7);
    assert_eq!(en_historial(&b, &e.id).len(), MAX_VERSIONES);
}

#[test]
fn eliminar_manda_el_historial_a_la_papelera() {
    let tmp = tempfile::tempdir().unwrap();
    let (b, papelera) = abrir(tmp.path());
    let e = importar(&b, "litematic_simple.litematic");
    let otro = importar(&b, "sponge_v2.schem");
    let (_, s) = b.cargar(&e.id).unwrap();
    let (region, p, _) = primer_bloque(&s);
    for estado in ["gold_block", "iron_block"] {
        b.aplicar_cambios(&e.id, &[cambio(&region, p, estado)], &[])
            .unwrap();
    }
    let (_, s) = b.cargar(&otro.id).unwrap();
    let (region, p, _) = primer_bloque(&s);
    b.aplicar_cambios(&otro.id, &[cambio(&region, p, "gold_block")], &[])
        .unwrap();

    b.eliminar(&e.id).unwrap();
    let recibidos = papelera.recibidos();
    let carpeta = b.carpeta_archivos().join("historial").join(&e.id);
    assert_eq!(
        recibidos,
        [
            b.carpeta_archivos().join(format!("{}.litematic", e.id)),
            carpeta.clone()
        ]
    );
    assert!(!carpeta.exists());
    // Las versiones del otro siguen.
    assert_eq!(b.historial(&otro.id).unwrap().len(), 1);
    assert_eq!(en_historial(&b, &otro.id).len(), 1);
    drop(b);
    let conn =
        rusqlite::Connection::open(tmp.path().join("datos").join("biblioteca.sqlite3")).unwrap();
    let ids: Vec<String> = conn
        .prepare("SELECT schematic_id FROM historial")
        .unwrap()
        .query_map([], |f| f.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(ids, [otro.id]);
}

#[test]
fn si_falla_la_escritura_la_copia_anterior_queda_intacta() {
    let tmp = tempfile::tempdir().unwrap();
    let datos = tmp.path().join("datos");
    let (b, _) = abrir(tmp.path());
    let e = importar(&b, "litematic_simple.litematic");
    let original = fs::read(ruta_copia(&b, &e)).unwrap();
    let hash = hash_en_indice(&datos, &e.id);
    let (_, s) = b.cargar(&e.id).unwrap();
    let (region, p, _) = primer_bloque(&s);
    let cambios = [cambio(&region, p, "minecraft:gold_block")];

    let comprobar_intacta = |b: &Biblioteca| {
        assert_eq!(fs::read(ruta_copia(b, &e)).unwrap(), original);
        assert_eq!(hash_en_indice(&datos, &e.id), hash);
        assert_eq!(b.listar().unwrap()[0], e);
        assert!(b.historial(&e.id).unwrap().is_empty());
        assert!(en_historial(b, &e.id).is_empty());
    };

    // 1. No se puede crear el temporal (hay una carpeta con su nombre).
    let tmp_edicion = b.carpeta_archivos().join(format!("{}.edicion.tmp", e.id));
    fs::create_dir(&tmp_edicion).unwrap();
    assert_eq!(
        b.aplicar_cambios(&e.id, &cambios, &[]).unwrap_err().tipo(),
        "io"
    );
    comprobar_intacta(&b);
    fs::remove_dir(&tmp_edicion).unwrap();

    // 2. No se puede crear la carpeta del historial (hay un archivo en su sitio).
    let dir_historial = b.carpeta_archivos().join("historial");
    fs::create_dir_all(&dir_historial).unwrap();
    fs::write(dir_historial.join(&e.id), b"estorbo").unwrap();
    assert_eq!(
        b.aplicar_cambios(&e.id, &cambios, &[]).unwrap_err().tipo(),
        "io"
    );
    comprobar_intacta(&b);
    assert!(!tmp_edicion.exists(), "el temporal no debe quedarse");
    fs::remove_file(dir_historial.join(&e.id)).unwrap();

    // Sin estorbos, la misma edición sale bien.
    let r = b.aplicar_cambios(&e.id, &cambios, &[]).unwrap();
    assert_eq!(r.cambios, 1);
    assert!(!tmp_edicion.exists());
}

#[test]
fn restaurar_un_contenido_que_ya_tiene_otra_entrada() {
    // Editar A, volver a importar el mismo archivo (ya no es duplicado: A ha
    // cambiado) y restaurar el original de A: dos entradas con el mismo
    // contenido no rompen la unicidad del hash.
    let tmp = tempfile::tempdir().unwrap();
    let (b, _) = abrir(tmp.path());
    let a = importar(&b, "litematic_simple.litematic");
    let (_, s) = b.cargar(&a.id).unwrap();
    let (region, p, _) = primer_bloque(&s);
    let v = b
        .aplicar_cambios(&a.id, &[cambio(&region, p, "gold_block")], &[])
        .unwrap()
        .version
        .unwrap();
    let otra = importar(&b, "litematic_simple.litematic");
    let r = b.restaurar_version(&a.id, v).unwrap();
    assert_eq!(r.entrada.tamano_archivo, otra.tamano_archivo);
    assert_eq!(b.listar().unwrap().len(), 2);
    // Importarlo otra vez lo detecta como duplicado.
    assert!(matches!(
        b.importar(&dato("litematic_simple.litematic")).unwrap(),
        Importacion::Duplicado(_)
    ));
}
