import opentype from "opentype.js"
import fs from "fs"
import iconv from "iconv-lite"

// Note that the .notdef glyph is required.
const notdefGlyph = new opentype.Glyph({
  name: '.notdef',
  unicode: 0,
  advanceWidth: 8,
  path: new opentype.Path()
})

const aPath = new opentype.Path()
aPath.moveTo(0, 0)
aPath.lineTo(150, 700)
aPath.lineTo(300, 0)
aPath.lineTo(150, 400)
// more drawing instructions...
const aGlyph = new opentype.Glyph({
  name: 'A',
  unicode: 65,
  advanceWidth: 500,
  path: aPath
})

const glyphs = [
  notdefGlyph,
  ...makeAsciiGlyphs(),
  ...makeGb2312Glyphs()
]
const font = new opentype.Font({
  familyName: 'WenQuXing-GB2312',
  styleName: 'Medium',
  unitsPerEm: 16,
  ascender: 16,
  descender: 0,
  glyphs,
})
font.download('WenQuXing-GB2312.ttf')

function makeAsciiGlyphs(): opentype.Glyph[] {
  const data = fs.readFileSync('../data/ascii8.dat')
  const glyphs = []

  for (let i = 1; i < 128; ++i) {
    glyphs.push(makeGlyph(i, data.slice(i * 16, i * 16 + 16), 8, 16))
  }

  return glyphs
}

function makeGb2312Glyphs(): opentype.Glyph[] {
  const data = fs.readFileSync('../data/gb16.dat')
  const glyphs = []

  for (let i = 0; i < 7614; ++i) {
    let byte1 = (i / 94 | 0) + 161;
    if (byte1 > 160 + 9) {
      byte1 += 6
    }
    const byte2 = i % 94 + 161;
    const cp = iconv.decode(Buffer.from([byte1, byte2]), 'gb2312').codePointAt(0)!
    if (cp === 0xfffd) {
      continue
    }
    glyphs.push(makeGlyph(cp, data.slice(i * 32, i * 32 + 32), 16, 16))
  }

  return glyphs
}

function makeGlyph(
  codepoint: number,
  data: Buffer,
  width: number,
  height: number
): opentype.Glyph {
  const byteWidth = (width + 7) >>> 3
  const unitSegments: Map<number, number[]> = new Map

  const getBitMask = (x: number) => 1 << (7 - x % 8)
  const getY = (y: number) => height - y - 1

  for (let y = 0; y < height; ++y) {
    for (let x = 0; x < width; ++x) {
      const bitMask = getBitMask(x)
      if ((data[getY(y) * byteWidth + (x >>> 3)] & bitMask) === 0) {
        continue
      }

      // top
      if (y === height - 1 || (data[getY(y + 1) * byteWidth + (x >>> 3)] & bitMask) === 0) {
        const from = (y + 1) * 100 + x
        const to = (y + 1) * 100 + (x + 1)
        initValue(unitSegments, from).push(to)
      }

      // bottom
      if (y === 0 || (data[getY(y - 1) * byteWidth + (x >>> 3)] & bitMask) === 0) {
        const from = y * 100 + (x + 1)
        const to = y * 100 + x
        initValue(unitSegments, from).push(to)
      }

      // left
      if (x === 0 || (data[getY(y) * byteWidth + ((x - 1) >>> 3)] & getBitMask(x - 1)) === 0) {
        const from = y * 100 + x
        const to = (y + 1) * 100 + x
        initValue(unitSegments, from).push(to)
      }

      // right
      if (x === width - 1 || (data[getY(y) * byteWidth + ((x + 1) >>> 3)] & getBitMask(x + 1)) === 0) {
        const from = (y + 1) * 100 + (x + 1)
        const to = y * 100 + (x + 1)
        initValue(unitSegments, from).push(to)
      }
    }
  }

  let path = new opentype.Path

  // connect segments
  while (unitSegments.size !== 0) {
    let from = unitSegments.keys().next().value
    let to = unitSegments.get(from)!.pop()!
    if (unitSegments.get(from)!.length === 0) {
      unitSegments.delete(from)
    }
    const segments: [number, number][] = []

    while (unitSegments.has(to)) {
      const nextToList = unitSegments.get(to)!
      let nextTo: number
      if (nextToList.length === 1) {
        nextTo = nextToList[0]
        unitSegments.delete(to)
      } else {
        if (direction(from, to, nextToList[0]) > 0) {
          nextTo = nextToList.shift()!
        } else {
          nextTo = nextToList.pop()!
        }
      }

      if (direction(from, to, nextTo) === 0) {
        // connect two segments
        to = nextTo
      } else {
        // new segment
        segments.push([from, to])
        from = to
        to = nextTo
      }
    }

    if (segments[0][0] !== to) {
      throw 'unreachable'
    }

    // if the last segment is collinear with the first segment, connect them.
    // otherwise simply discard the last segment.
    if (direction(from, segments[0][0], segments[0][1]) === 0) {
      segments[0][0] = from
      segments.pop()
    }

    path.moveTo(segments[0][0] % 100, segments[0][0] / 100 | 0)

    for (const [, to] of segments) {
      path.lineTo(to % 100, to / 100 | 0)
    }
  }

  return new opentype.Glyph({
    name: 'U+' + codepoint.toString(16).padStart(4),
    unicode: codepoint,
    advanceWidth: width,
    path,
  })
}

function initValue<K, V>(map: Map<K, V[]>, key: K): V[] {
  if (!map.has(key)) {
    map.set(key, [])
  }
  return map.get(key)!
}

/**
 * @returns clockwise if positive; counter-clockwise if negative; collinear if zero.
 */
function direction(p1: number, p2: number, p3: number): number {
  const x1 = p3 % 100 - p1 % 100, y1 = (p3 / 100 | 0) - (p1 / 100 | 0)
  const x2 = p2 % 100 - p1 % 100, y2 = (p2 / 100 | 0) - (p1 / 100 | 0)
  // cross product
  return x1 * y2 - x2 * y1
}