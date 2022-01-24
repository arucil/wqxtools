import opentype from "opentype.js"
import fs from "fs"
import iconv from "iconv-lite"

const SCALE = 100

// Note that the .notdef glyph is required.
const notdefPath = new opentype.Path()
notdefPath.moveTo(0, 0)
notdefPath.lineTo(0, 1500)
notdefPath.lineTo(1500, 1500)
notdefPath.lineTo(1500, 0)
notdefPath.moveTo(120, 70)
notdefPath.lineTo(1380, 70)
notdefPath.lineTo(750, 700)
notdefPath.moveTo(70, 120)
notdefPath.lineTo(700, 750)
notdefPath.lineTo(70, 1380)
notdefPath.moveTo(120, 1430)
notdefPath.lineTo(750, 800)
notdefPath.lineTo(1380, 1430)
notdefPath.moveTo(1430, 1380)
notdefPath.lineTo(800, 750)
notdefPath.lineTo(1430, 120)
const notdefGlyph = new opentype.Glyph({
  name: '.notdef',
  unicode: 0,
  advanceWidth: 16 * SCALE,
  path: notdefPath
})

// gb2312
{
  console.log('generating WenQuXing.ttf...')
  const glyphs = [
    notdefGlyph,
    ...makeAsciiGlyphs(),
    ...makeGb2312Glyphs(),
    ...makeIconGlyphs()
  ]
  const font = new opentype.Font({
    familyName: 'WenQuXing',
    styleName: 'Regular',
    unitsPerEm: 16.4 * SCALE,
    ascender: 15.9 * SCALE,
    descender: 0,
    glyphs,
  })
  font.download('WenQuXing.ttf')
}

function makeAsciiGlyphs(): opentype.Glyph[] {
  const data = fs.readFileSync('../data/ascii_16.dat')
  const glyphs = []

  for (let i = 1; i < 128; ++i) {
    glyphs.push(makeGlyph(i, data.slice(i * 16, i * 16 + 16), 8, 16))
  }

  return glyphs
}

function makeGb2312Glyphs(): opentype.Glyph[] {
  const data = fs.readFileSync('../data/gb2312_16.dat')
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

function makeIconGlyphs(): opentype.Glyph[] {
  const data = fs.readFileSync('../data/icon_16.dat')
  const glyphs = []

  for (let i = 0; i < 527; ++i) {
    const cp = 0xe000 + i
    glyphs.push(makeGlyph(cp, data.slice(i * 32, i * 32 + 32), 16, 16))
  }

  for (let i = 0xe300; i <= 0xeaff; ++i) {
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
    const y1 = y - (codepoint < 256 ? 2 : 1)
    for (let x = 0; x < width; ++x) {
      const bitMask = getBitMask(x)
      if ((data[getY(y) * byteWidth + (x >>> 3)] & bitMask) === 0) {
        continue
      }

      // top
      if (y === height - 1 || (data[getY(y + 1) * byteWidth + (x >>> 3)] & bitMask) === 0) {
        const from = (y1 + 1) * 100 + x
        const to = (y1 + 1) * 100 + (x + 1)
        initValue(unitSegments, from).push(to)
      }

      // bottom
      if (y === 0 || (data[getY(y - 1) * byteWidth + (x >>> 3)] & bitMask) === 0) {
        const from = y1 * 100 + (x + 1)
        const to = y1 * 100 + x
        initValue(unitSegments, from).push(to)
      }

      // left
      if (x === 0 || (data[getY(y) * byteWidth + ((x - 1) >>> 3)] & getBitMask(x - 1)) === 0) {
        const from = y1 * 100 + x
        const to = (y1 + 1) * 100 + x
        initValue(unitSegments, from).push(to)
      }

      // right
      if (x === width - 1 || (data[getY(y) * byteWidth + ((x + 1) >>> 3)] & getBitMask(x + 1)) === 0) {
        const from = (y1 + 1) * 100 + (x + 1)
        const to = y1 * 100 + (x + 1)
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

    path.moveTo(segments[0][0] % 100 * SCALE, (segments[0][0] / 100 | 0) * SCALE)

    for (const [, to] of segments) {
      path.lineTo(to % 100 * SCALE, (to / 100 | 0) * SCALE)
    }
  }

  return new opentype.Glyph({
    name: 'U+' + codepoint.toString(16).padStart(4, '0'),
    unicode: codepoint,
    advanceWidth: width * SCALE,
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
