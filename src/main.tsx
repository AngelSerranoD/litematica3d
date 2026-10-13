// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { aplicarTemaRecordado } from "./estado/ajustes";
import "./index.css";

// El tema de la última sesión antes del primer fotograma (los ajustes de
// verdad llegan de Rust un momento después).
aplicarTemaRecordado();

// Las funciones de Motion (layout incluido) viven en su propio chunk, fuera del
// de entrada, y se esperan antes del primer render: con LazyMotion asíncrono las
// láminas de la biblioteca se quedaban en su estado «oculta» al arrancar.
const { default: funciones } = await import("./estilos/funcionesMotion");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App funciones={funciones} />
  </React.StrictMode>,
);
