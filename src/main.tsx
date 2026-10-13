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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
