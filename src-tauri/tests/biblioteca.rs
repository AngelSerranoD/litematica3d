// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

//! Pruebas de la biblioteca con un directorio temporal.

use std::fs;
use std::path::{Path, PathBuf};

use litematica3d_lib::biblioteca::{Biblioteca, Importacion};
use litematica3d_lib::error::ErrorApp;
use litematica3d_lib::schematic::Formato;

fn dato(nombre: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("datos")
        .join(nombre)
}

/// Archivos dentro de la carpeta de copias.
fn copias(b: &Biblioteca) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(b.carpeta_archivos())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn importado(r: Result<Importacion, ErrorApp>) -> litematica3d_lib::biblioteca::Entrada {
    match r.unwrap() {
        Importacion::Importado(e) => e,
        Importacion::Duplicado(e) => panic!("no debía ser duplicado: {}", e.nombre),
    }
}

#[test]
fn importar_listar_y_persistencia() {
    let tmp = tempfile::tempdir().unwrap();
    let id_simple;
    {
        let b = Biblioteca::abrir(tmp.path()).unwrap();
        let e = importado(b.importar(&dato("litematic_simple.litematic")));
        assert_eq!(e.nombre, "Casita de prueba");
        assert_eq!(e.formato, Formato::Litematic);
        assert_eq!(e.total_bloques, 241);
        assert_eq!(e.regiones, 1);
        assert_eq!(e.autor.as_deref(), Some("Ángel Serrano"));
        assert_eq!(
            e.nombre_original.as_deref(),
            Some("litematic_simple.litematic")
        );
        assert_eq!(
            e.tamano_archivo,
            fs::metadata(dato("litematic_simple.litematic"))
                .unwrap()
                .len()
        );
        assert_eq!(copias(&b), [format!("{}.litematic", e.id)]);
        id_simple = e.id;

        // Sin Metadata.Name: el nombre sale del archivo sin extensión.
        let m = importado(b.importar(&dato("mcedit.schematic")));
        assert_eq!(m.nombre, "mcedit");
        assert_eq!(m.formato, Formato::Mcedit);
        assert_eq!(copias(&b).len(), 2);
    }

    // Reabrir la conexión: los datos siguen ahí, de más reciente a más antiguo.
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let lista = b.listar().unwrap();
    assert_eq!(lista.len(), 2);
    assert_eq!(lista[0].nombre, "mcedit");
    assert_eq!(lista[1].id, id_simple);
    assert!(lista[0].fecha_importacion >= lista[1].fecha_importacion);
    assert!(!copias(&b).iter().any(|c| c.ends_with(".tmp")));
}

#[test]
fn duplicado_por_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let original = importado(b.importar(&dato("sponge_v2.schem")));

    // El mismo archivo con otro nombre y otra extensión sigue siendo duplicado.
    let otra = tmp.path().join("Copia.SCHEMATIC");
    fs::copy(dato("sponge_v2.schem"), &otra).unwrap();
    for ruta in [dato("sponge_v2.schem"), otra] {
        match b.importar(&ruta).unwrap() {
            Importacion::Duplicado(e) => assert_eq!(e, original),
            Importacion::Importado(_) => panic!("debía ser duplicado"),
        }
    }
    assert_eq!(copias(&b).len(), 1);
    assert_eq!(b.listar().unwrap().len(), 1);
}

#[test]
fn un_error_no_deja_rastro() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();

    let e = b.importar(&dato("corrupto.litematic")).unwrap_err();
    assert_eq!(e.tipo(), "archivoCorrupto");
    assert!(e.to_string().contains("«corrupto.litematic»"), "{e}");

    let e = b.importar(&dato("basura.schem")).unwrap_err();
    assert_eq!(e.tipo(), "formatoDesconocido");

    let e = b
        .importar(&tmp.path().join("no_existe.litematic"))
        .unwrap_err();
    assert_eq!(e.tipo(), "io");

    assert!(copias(&b).is_empty());
    assert!(b.listar().unwrap().is_empty());

    // Un error no impide importar después.
    importado(b.importar(&dato("estructura.nbt")));
    assert_eq!(b.listar().unwrap().len(), 1);
}

#[test]
fn renombrar() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let e = importado(b.importar(&dato("sponge_v3.schem")));

    let r = b.renombrar(&e.id, "  Torre del homenaje  ").unwrap();
    assert_eq!(r.nombre, "Torre del homenaje");
    assert_eq!(b.listar().unwrap()[0].nombre, "Torre del homenaje");

    assert_eq!(
        b.renombrar(&e.id, "   ").unwrap_err().tipo(),
        "nombreInvalido"
    );
    assert_eq!(
        b.renombrar(&e.id, &"ñ".repeat(201)).unwrap_err().tipo(),
        "nombreInvalido"
    );
    assert_eq!(
        b.renombrar(&e.id, &"ñ".repeat(200))
            .unwrap()
            .nombre
            .chars()
            .count(),
        200
    );
    assert_eq!(
        b.renombrar("no-existe", "Algo").unwrap_err().tipo(),
        "noEncontrado"
    );
}

#[test]
fn obtener_detalle_y_errores() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let e = importado(b.importar(&dato("litematic_multi.litematic")));

    let d = b.obtener(&e.id).unwrap();
    assert_eq!(d.entrada, e);
    let nombres: Vec<&str> = d.regiones.iter().map(|r| r.nombre.as_str()).collect();
    assert_eq!(nombres, ["negativa", "positiva"]);
    assert_eq!(d.regiones[0].origen, [-4, 0, 0]);
    assert_eq!(d.regiones.iter().map(|r| r.total_bloques).sum::<u64>(), 86);

    let json = serde_json::to_value(&d).unwrap();
    assert_eq!(json["entrada"]["totalBloques"], 86);
    assert_eq!(json["entrada"]["formato"], "litematic");
    assert_eq!(
        json["regiones"][0]["tamanoPaleta"],
        d.regiones[0].tamano_paleta
    );

    let err = b.obtener("no-existe").unwrap_err();
    assert_eq!(err.tipo(), "noEncontrado");
    assert_eq!(serde_json::to_value(&err).unwrap()["tipo"], "noEncontrado");

    // Copia borrada a mano: error claro, no panic.
    borrar_copia(b.carpeta_archivos(), &e.id);
    assert_eq!(b.obtener(&e.id).unwrap_err().tipo(), "archivoPerdido");
}

/// Quita la copia interna (archivo del directorio temporal del test).
fn borrar_copia(carpeta: &Path, id: &str) {
    for c in fs::read_dir(carpeta).unwrap() {
        let c = c.unwrap();
        if c.file_name().to_string_lossy().starts_with(id) {
            fs::remove_file(c.path()).unwrap();
        }
    }
}

#[test]
fn eliminar_cuando_el_archivo_ya_no_existe() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let e = importado(b.importar(&dato("estructura_palettes.nbt")));
    borrar_copia(b.carpeta_archivos(), &e.id);

    b.eliminar(&e.id).unwrap();
    assert!(b.listar().unwrap().is_empty());
    assert_eq!(b.eliminar(&e.id).unwrap_err().tipo(), "noEncontrado");

    // Tras eliminarlo se puede volver a importar.
    importado(b.importar(&dato("estructura_palettes.nbt")));
}

/// Mueve un archivo de verdad a la Papelera de Windows: se ejecuta a mano con
/// `cargo test -- --ignored`.
#[test]
#[ignore = "deja un archivo en la Papelera del sistema"]
fn eliminar_mueve_a_la_papelera() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let e = importado(b.importar(&dato("sin_comprimir.nbt")));
    b.eliminar(&e.id).unwrap();
    assert!(copias(&b).is_empty());
    assert!(b.listar().unwrap().is_empty());
}

// ------------------------------------------------- comprobación (esquema v2)

#[test]
fn marcar_desmarcar_y_persistencia() {
    let tmp = tempfile::tempdir().unwrap();
    let id;
    {
        let b = Biblioteca::abrir(tmp.path()).unwrap();
        assert!(b.claves_ajenas_activas().unwrap());
        id = importado(b.importar(&dato("litematic_simple.litematic"))).id;
        assert!(b.comprobacion(&id).unwrap().is_empty());
        b.marcar(&id, "minecraft:stone", true).unwrap();
        b.marcar(&id, "minecraft:oak_planks", true).unwrap();
        // Marcar dos veces o desmarcar lo no marcado no es un error.
        b.marcar(&id, "minecraft:stone", true).unwrap();
        b.marcar(&id, "minecraft:dirt", false).unwrap();
        assert_eq!(
            b.comprobacion(&id).unwrap(),
            ["minecraft:oak_planks", "minecraft:stone"]
        );
        b.marcar(&id, "minecraft:oak_planks", false).unwrap();
    }
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    assert_eq!(b.comprobacion(&id).unwrap(), ["minecraft:stone"]);

    assert_eq!(
        b.comprobacion("no-existe").unwrap_err().tipo(),
        "noEncontrado"
    );
    assert_eq!(
        b.marcar("no-existe", "minecraft:stone", true)
            .unwrap_err()
            .tipo(),
        "noEncontrado"
    );
    assert_eq!(
        b.marcar(&id, "  ", true).unwrap_err().tipo(),
        "nombreInvalido"
    );
}

#[test]
fn eliminar_borra_su_comprobacion() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let uno = importado(b.importar(&dato("litematic_simple.litematic"))).id;
    let otro = importado(b.importar(&dato("sponge_v2.schem"))).id;
    b.marcar(&uno, "minecraft:stone", true).unwrap();
    b.marcar(&otro, "minecraft:stone", true).unwrap();
    borrar_copia(b.carpeta_archivos(), &uno);
    b.eliminar(&uno).unwrap();
    drop(b);

    let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
    let filas: Vec<String> = conn
        .prepare("SELECT schematic_id FROM comprobacion")
        .unwrap()
        .query_map([], |f| f.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(filas, [otro]);
}

#[test]
fn borrado_en_cascada_por_clave_ajena() {
    // Aunque se borre la fila del schematic por fuera de `eliminar`, la
    // clave ajena se lleva su comprobación.
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let id = importado(b.importar(&dato("litematic_simple.litematic"))).id;
    b.marcar(&id, "minecraft:stone", true).unwrap();
    drop(b);
    let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    conn.execute("DELETE FROM schematics WHERE id = ?1", [&id])
        .unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM comprobacion", [], |f| f.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn migra_una_biblioteca_v1_con_datos() {
    let tmp = tempfile::tempdir().unwrap();
    {
        // Esquema v1 tal cual lo creaba la versión anterior, con una fila.
        let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
        conn.execute_batch(
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
             INSERT INTO schematics VALUES ('viejo', 'Casa vieja', NULL, NULL,
                 'litematic', 3, 4, 5, 60, 1, NULL, NULL, 1700000000000,
                 'viejo.litematic', 'casa.litematic', 123, 'abc');
             PRAGMA user_version = 1;",
        )
        .unwrap();
    }
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let lista = b.listar().unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].nombre, "Casa vieja");
    assert!(b.comprobacion("viejo").unwrap().is_empty());
    b.marcar("viejo", "minecraft:stone", true).unwrap();
    assert_eq!(b.comprobacion("viejo").unwrap(), ["minecraft:stone"]);
    drop(b);

    let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
    let v: i32 = conn
        .pragma_query_value(None, "user_version", |f| f.get(0))
        .unwrap();
    assert_eq!(v, 3);
    // Abrirla otra vez no vuelve a migrar.
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    assert_eq!(b.comprobacion("viejo").unwrap(), ["minecraft:stone"]);
}

// ------------------------------------------------- miniaturas (esquema v3)

/// PNG RGBA de `ancho` × `alto` píxeles, transparente.
fn png(ancho: u32, alto: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, ancho, alto);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&vec![0; (ancho * alto * 4) as usize])
            .unwrap();
    }
    out
}

#[test]
fn guardar_regenerar_e_invalidar_miniatura() {
    let tmp = tempfile::tempdir().unwrap();
    let id;
    {
        let b = Biblioteca::abrir(tmp.path()).unwrap();
        let e = importado(b.importar(&dato("litematic_simple.litematic")));
        assert_eq!(e.miniatura, None);
        id = e.id;

        let e = b
            .guardar_miniatura(&id, &png(512, 512), "r1|1.21.4")
            .unwrap();
        let m = e.miniatura.clone().unwrap();
        assert_eq!(m.texturas.as_deref(), Some("r1|1.21.4"));
        assert_eq!(Path::new(&m.ruta), b.ruta_miniatura(&id));
        assert!(m.ruta.ends_with(&format!("{id}.miniatura.png")));
        assert_eq!(fs::read(&m.ruta).unwrap(), png(512, 512));
        assert_eq!(b.listar().unwrap()[0].miniatura, Some(m.clone()));
        // Junto a la copia y sin temporales.
        assert_eq!(
            copias(&b),
            [format!("{id}.litematic"), format!("{id}.miniatura.png")]
        );

        // Cambian las texturas: obsoleta, pero el PNG sigue ahí.
        assert_eq!(b.invalidar_miniaturas().unwrap(), 1);
        let m2 = b.listar().unwrap()[0].miniatura.clone().unwrap();
        assert_eq!(m2.texturas, None);
        assert_eq!(m2.fecha, m.fecha);
        assert!(Path::new(&m2.ruta).exists());

        // Regenerarla sustituye el archivo.
        let e = b.guardar_miniatura(&id, &png(256, 256), "r1|-").unwrap();
        assert_eq!(e.miniatura.unwrap().texturas.as_deref(), Some("r1|-"));
        assert_eq!(fs::read(b.ruta_miniatura(&id)).unwrap(), png(256, 256));
    }
    // Persiste al reabrir.
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let m = b.listar().unwrap()[0].miniatura.clone().unwrap();
    assert_eq!(m.texturas.as_deref(), Some("r1|-"));
}

#[test]
fn miniatura_invalida_no_deja_rastro() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let id = importado(b.importar(&dato("litematic_simple.litematic"))).id;

    let mut cortado = png(128, 128);
    cortado.truncate(cortado.len() - 20);
    let mut enorme = png(64, 64);
    enorme.resize(4 * 1024 * 1024 + 1, 0);
    for (que, bytes) in [
        ("no es PNG", b"GIF89a....".to_vec()),
        ("vacío", Vec::new()),
        ("cortado", cortado),
        ("no cuadrado", png(512, 256)),
        ("demasiado pequeño", png(32, 32)),
        ("demasiado grande", png(2048, 2048)),
        ("más de 4 MiB", enorme),
    ] {
        let e = b.guardar_miniatura(&id, &bytes, "r1|-").unwrap_err();
        assert_eq!(e.tipo(), "miniaturaInvalida", "{que}");
    }
    let larga = "x".repeat(101);
    for clave in ["", "con espacio", "ñ", larga.as_str()] {
        let e = b.guardar_miniatura(&id, &png(64, 64), clave).unwrap_err();
        assert_eq!(e.tipo(), "miniaturaInvalida", "{clave:?}");
    }
    // Ids que no son de la biblioteca: ni rutas ni ids inventados.
    for otro in ["no-existe", "../fuera", r"..\fuera", "C:/x", ""] {
        let e = b.guardar_miniatura(otro, &png(64, 64), "r1|-").unwrap_err();
        assert_eq!(e.tipo(), "noEncontrado", "{otro}");
    }
    assert_eq!(copias(&b), [format!("{id}.litematic")]);
    assert_eq!(b.listar().unwrap()[0].miniatura, None);
}

#[test]
fn eliminar_sin_archivos_quita_la_miniatura_del_indice() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let id = importado(b.importar(&dato("estructura_palettes.nbt"))).id;
    b.guardar_miniatura(&id, &png(64, 64), "r1|-").unwrap();
    // Copia y miniatura borradas a mano: solo queda la fila.
    borrar_copia(b.carpeta_archivos(), &id);
    b.eliminar(&id).unwrap();
    assert!(b.listar().unwrap().is_empty());
    assert!(copias(&b).is_empty());
}

/// Manda copia y miniatura a la Papelera de Windows: a mano con
/// `cargo test -- --ignored`.
#[test]
#[ignore = "deja dos archivos en la Papelera del sistema"]
fn eliminar_manda_tambien_la_miniatura_a_la_papelera() {
    let tmp = tempfile::tempdir().unwrap();
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let id = importado(b.importar(&dato("sin_comprimir.nbt"))).id;
    b.guardar_miniatura(&id, &png(64, 64), "r1|-").unwrap();
    b.eliminar(&id).unwrap();
    assert!(copias(&b).is_empty());
}

#[test]
fn migra_una_biblioteca_v2_con_comprobacion() {
    let tmp = tempfile::tempdir().unwrap();
    {
        // Esquema v2 tal cual: el de v1 más la tabla `comprobacion`.
        let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
        conn.execute_batch(
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
             CREATE TABLE comprobacion (
                 schematic_id TEXT NOT NULL REFERENCES schematics (id) ON DELETE CASCADE,
                 item TEXT NOT NULL,
                 PRIMARY KEY (schematic_id, item)
             ) WITHOUT ROWID;
             INSERT INTO schematics VALUES ('viejo', 'Casa vieja', NULL, NULL,
                 'litematic', 3, 4, 5, 60, 1, NULL, NULL, 1700000000000,
                 'viejo.litematic', 'casa.litematic', 123, 'abc');
             INSERT INTO comprobacion VALUES ('viejo', 'minecraft:stone');
             PRAGMA user_version = 2;",
        )
        .unwrap();
    }
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let lista = b.listar().unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0].nombre, "Casa vieja");
    assert_eq!(lista[0].miniatura, None);
    assert_eq!(b.comprobacion("viejo").unwrap(), ["minecraft:stone"]);
    let e = b.guardar_miniatura("viejo", &png(64, 64), "r1|-").unwrap();
    assert!(e.miniatura.is_some());
    drop(b);

    let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
    let v: i32 = conn
        .pragma_query_value(None, "user_version", |f| f.get(0))
        .unwrap();
    assert_eq!(v, 3);
    drop(conn);
    // Reabrir no vuelve a migrar (el ALTER TABLE fallaría por columna repetida).
    let b = Biblioteca::abrir(tmp.path()).unwrap();
    let m = b.listar().unwrap()[0].miniatura.clone().unwrap();
    assert_eq!(m.texturas.as_deref(), Some("r1|-"));
}

#[test]
fn una_biblioteca_mas_nueva_no_se_abre() {
    let tmp = tempfile::tempdir().unwrap();
    drop(Biblioteca::abrir(tmp.path()).unwrap());
    let conn = rusqlite::Connection::open(tmp.path().join("biblioteca.sqlite3")).unwrap();
    conn.pragma_update(None, "user_version", 4).unwrap();
    drop(conn);
    let e = Biblioteca::abrir(tmp.path()).err().unwrap();
    assert_eq!(e.tipo(), "versionBiblioteca");
}
