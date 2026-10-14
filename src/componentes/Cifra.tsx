// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { gruposDeMiles } from "../logica/formato";
import "./Cifra.css";

/**
 * Número en JetBrains Mono con los miles separados por un espacio fino.
 *
 * El espacio fino no es un carácter: en una fuente monoespaciada U+2009
 * ocupa una celda entera. Cada grupo va en su propio span y el hueco es un
 * margen de 0,2 em; así el lector de pantalla lee el número de corrido.
 */
export function Cifra({ valor, className }: { valor: number; className?: string }) {
  const grupos = gruposDeMiles(valor);
  return (
    <span className={className ? `cifras cifra ${className}` : "cifras cifra"}>
      {grupos.map((g, i) => (
        <span key={i} className="cifra__grupo">
          {g}
        </span>
      ))}
    </span>
  );
}
