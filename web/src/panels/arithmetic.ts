/**
 * What a panel's number field reads (plan 2026-11-25, stage 3): a number, or arithmetic on numbers
 * (`50+25`, `100/2`, `(10 + 4) * 2`, `-5`), each number optionally with a unit (`10 mm`, `1in`,
 * `12pt`, `3px`, `2pc`, `1cm`) converted to the field's unit. A field shows its value in the
 * document's unit; a number without a unit is in that unit. Pure, so `node --test` checks it.
 */

/** Points per unit, by the abbreviation typed. */
const UNIT_POINTS: Readonly<Record<string, number>> = {
  pt: 1,
  px: 1,
  pc: 12,
  in: 72,
  '"': 72,
  ft: 864,
  yd: 2592,
  mm: 72 / 25.4,
  cm: 72 / 2.54,
  m: 7200 / 2.54,
}

/**
 * The value of `text`, in a field whose unit is `pointsPerUnit` points (1 for points, and for
 * fields that are not lengths, where units are refused); null when it is not arithmetic on numbers.
 */
export function evaluate(text: string, pointsPerUnit = 1, lengths = true): number | null {
  const source = text.trim().replace(/,/g, '.').replace(/×/g, '*').replace(/÷/g, '/')
  if (!source) return null
  let at = 0
  const space = () => {
    while (source[at] === ' ') at += 1
  }
  const fail = (): never => {
    throw new Error('not arithmetic')
  }
  const number = (): number => {
    space()
    const match = /^(\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?/i.exec(source.slice(at))
    if (!match) return fail()
    at += match[0].length
    let value = Number(match[0])
    space()
    const unit = /^(pt|px|pc|in|ft|yd|mm|cm|m|")/i.exec(source.slice(at))
    if (unit) {
      if (!lengths) return fail()
      at += unit[0].length
      value = (value * (UNIT_POINTS[unit[0].toLowerCase()] ?? 1)) / pointsPerUnit
    }
    return value
  }
  const factor = (): number => {
    space()
    const char = source[at]
    if (char === '-' || char === '+') {
      at += 1
      const inner = factor()
      return char === '-' ? -inner : inner
    }
    if (char === '(') {
      at += 1
      const inner = sum()
      space()
      if (source[at] !== ')') return fail()
      at += 1
      return inner
    }
    return number()
  }
  const product = (): number => {
    let value = factor()
    for (;;) {
      space()
      const op = source[at]
      if (op !== '*' && op !== '/') return value
      at += 1
      const next = factor()
      if (op === '/' && next === 0) return fail()
      value = op === '*' ? value * next : value / next
    }
  }
  const sum = (): number => {
    let value = product()
    for (;;) {
      space()
      const op = source[at]
      if (op !== '+' && op !== '-') return value
      at += 1
      const next = product()
      value = op === '+' ? value + next : value - next
    }
  }
  try {
    const value = sum()
    space()
    if (at !== source.length || !Number.isFinite(value)) return null
    return value
  } catch {
    return null
  }
}

/** `value` rounded to `decimals` places, written without trailing zeros. */
export function formatNumber(value: number, decimals: number) {
  const scale = 10 ** decimals
  return String(Math.round(value * scale) / scale)
}
