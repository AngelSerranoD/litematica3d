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
