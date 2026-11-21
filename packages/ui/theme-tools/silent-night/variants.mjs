/**
 * The curated snow caps. Each entry is the seed of its random stream plus the
 * choices pinned by hand: 3 dustings (thick 4-6.5), 7 medium (10-16) and
 * 6 heavy (17-20); every mound count 1-4 four times; 4 spill over the left
 * corner and 4 over the right; icicles and glints on most.
 */
export const VARIANTS = [
  {
    seed: 101,
    thick: 18,
    mounds: 2,
    floor: 0.2,
    spill: "l",
    icicles: 3,
    glints: 2
  },
  {
    seed: 2024,
    thick: 12.5,
    mounds: 3,
    floor: 0.25,
    spill: "",
    icicles: 0,
    glints: 1
  },
  {
    seed: 303,
    thick: 5,
    mounds: 1,
    floor: 0.35,
    spill: "r",
    icicles: 2,
    glints: 0
  },
  {
    seed: 404,
    thick: 20,
    mounds: 4,
    floor: 0.15,
    spill: "",
    icicles: 4,
    glints: 3
  },
  {
    seed: 505,
    thick: 14,
    mounds: 2,
    floor: 0.25,
    spill: "r",
    icicles: 0,
    glints: 2
  },
  {
    seed: 6068,
    thick: 15.5,
    mounds: 1,
    floor: 0.22,
    spill: "",
    icicles: 1,
    glints: 0
  },
  {
    seed: 707,
    thick: 19,
    mounds: 3,
    floor: 0.22,
    spill: "l",
    icicles: 2,
    glints: 1
  },
  {
    seed: 808,
    thick: 16,
    mounds: 4,
    floor: 0.2,
    spill: "",
    icicles: 0,
    glints: 2
  },
  {
    seed: 909,
    thick: 6,
    mounds: 2,
    floor: 0.4,
    spill: "",
    icicles: 0,
    glints: 1
  },
  {
    seed: 10107,
    thick: 14,
    mounds: 3,
    floor: 0.06,
    spill: "r",
    icicles: 3,
    glints: 0
  },
  {
    seed: 1111,
    thick: 17,
    mounds: 1,
    floor: 0.12,
    spill: "l",
    icicles: 0,
    glints: 3
  },
  {
    seed: 1212,
    thick: 13,
    mounds: 4,
    floor: 0.28,
    spill: "",
    icicles: 2,
    glints: 1
  },
  {
    seed: 1313,
    thick: 20,
    mounds: 2,
    floor: 0.1,
    spill: "r",
    icicles: 5,
    glints: 2
  },
  {
    seed: 1414,
    thick: 15,
    mounds: 3,
    floor: 0.3,
    spill: "l",
    icicles: 0,
    glints: 0
  },
  {
    seed: 1515,
    thick: 4.5,
    mounds: 4,
    floor: 0.45,
    spill: "",
    icicles: 1,
    glints: 2
  },
  {
    seed: 1616,
    thick: 18.5,
    mounds: 1,
    floor: 0.2,
    spill: "",
    icicles: 0,
    glints: 1
  }
]
