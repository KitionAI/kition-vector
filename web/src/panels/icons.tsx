import { forwardRef, type CSSProperties } from 'react'

/**
 * Kition Vector's own panel icons (plan 2026-11-25, stage 3), drawn here in code as 24-unit SVG in
 * Tessera's line style, for what Tessera's set has no mark of: the six Distribute Objects and two
 * Distribute Spacing buttons, and four Pathfinder operations (Merge, Crop, Outline, Minus Back).
 * Original work for Kition Vector (registered in `docs/attribution.md`).
 */
type Props = { size?: number | string; strokeWidth?: number | string; color?: string; className?: string; style?: CSSProperties; 'aria-hidden'?: boolean | 'true' | 'false' }

function icon(name: string, body: (string | { d: string })[]) {
  const Icon = forwardRef<SVGSVGElement, Props>(function Icon({ size = 24, strokeWidth = 2, color = 'currentColor', className, style, ...rest }, ref) {
    return (
      <svg ref={ref} xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" className={className} style={style} aria-hidden={rest['aria-hidden'] ?? true}>
        {body.map((part, index) => (typeof part === 'string' ? <path key={index} d={part} /> : <path key={index} d={part.d} fill={color} stroke="none" fillOpacity={0.4} />))}
      </svg>
    )
  })
  Icon.displayName = name
  return Icon
}

// Distribute: three objects and the edge or centre line each is spaced by.
export const DistributeTop = icon('DistributeTop', ['M3 4h18', 'M3 11h18', 'M3 18h18', 'M5 4v4h4V4', 'M10 11v5h4v-5', 'M16 18v3h3v-3'])
export const DistributeMiddleV = icon('DistributeMiddleV', ['M3 5h18', 'M3 12h18', 'M3 19h18', 'M5 3h4v4H5z', 'M10 9.5h4v5h-4z', 'M16 17.5h3v3h-3z'])
export const DistributeBottom = icon('DistributeBottom', ['M3 6h18', 'M3 13h18', 'M3 20h18', 'M5 6V2h4v4', 'M10 13V8h4v5', 'M16 20v-3h3v3'])
export const DistributeLeft = icon('DistributeLeft', ['M4 3v18', 'M11 3v18', 'M18 3v18', 'M4 5h4v4H4', 'M11 10h5v5h-5', 'M18 16h3v3h-3'])
export const DistributeMiddleH = icon('DistributeMiddleH', ['M5 3v18', 'M12 3v18', 'M19 3v18', 'M3 4h4v4H3z', 'M9.5 10h5v5h-5z', 'M17.5 16h3v3h-3z'])
export const DistributeRight = icon('DistributeRight', ['M6 3v18', 'M13 3v18', 'M20 3v18', 'M6 4H2v4h4', 'M13 10H8v5h5', 'M20 16h-3v3h3'])
// Distribute Spacing: equal gaps between three objects, across or down.
export const SpaceHorizontal = icon('SpaceHorizontal', ['M3 6h4v12H3z', 'M10 6h4v12h-4z', 'M17 6h4v12h-4z', 'M7 21h3', 'M14 21h3'])
export const SpaceVertical = icon('SpaceVertical', ['M6 3h12v4H6z', 'M6 10h12v4H6z', 'M6 17h12v4H6z', 'M21 7v3', 'M21 14v3'])
// Pathfinder: two overlapping squares, what the operation keeps filled.
export const PathfinderMerge = icon('PathfinderMerge', ['M3 3h11v11H3z', 'M10 10h11v11H10z', { d: 'M3 3h11v7h-4v4H3z' }, { d: 'M10 10h11v11H10z' }])
export const PathfinderCrop = icon('PathfinderCrop', ['M3 3h11v11H3z', 'M10 10h11v11H10z', { d: 'M10 10h4v4h-4z' }])
export const PathfinderOutline = icon('PathfinderOutline', ['M3 3h11v7', 'M10 14H3V3', 'M10 10h11v11H10z', 'M10 10h4v4h-4'])
export const PathfinderMinusBack = icon('PathfinderMinusBack', ['M3 3h11v11H3z', 'M10 10h11v11H10z', { d: 'M14 10h7v11H10v-7h4z' }])
