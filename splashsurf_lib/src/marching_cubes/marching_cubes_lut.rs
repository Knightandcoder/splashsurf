//! Classic marching cubes cases.
//! There are 256 possible combinations of the above/below iso-surface states for the 8 vertices
//! of a cube. The following lookup table maps each combination to the corresponding triangulation.
//!
//! The index for a case is obtained with a bitfield of size 8, where a bit value of 1
//! indicates that the corresponding vertex of the cube is above the iso-surface threshold.
//! Reversing the order of the bit pattern and interpreting it as an integer yields the case index.
//!
//! For each case, the triangulation is represented by a 16 element array containing successive
//! index triplets for each required triangle. The indices refer to the corresponding edges that
//! are intersected by the triangle. Each case has at most three triangles and unused entries of the
//! 16 element arrays are filled with -1 entries for padding.
//!
//! Example:
//!   - Vertex 0 and 2 are above the iso-surface threshold.
//!   - The corresponding bit pattern is `10100000`, the corresponding index is 5
//!   - The case with index 5 reads `[ 0,  8,  3,  1,  2, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1]`
//!   - The triangulation is given by the two triangles `[0, 8, 3]` and `[1, 2, 10]`, with vertices
//!     on the edges identified by the given indices
//!
//! Note that the raw table apparently uses a left-handed coordinate system and accordingly a
//! clockwise winding order of the triangles. To avoid producing meshes with normals pointing into
//! the reconstructed surface, the resulting triangles have to be flipped. This is already taken
//! into account by the [`marching_cubes_triangulation_iter`] function.
//!
//! Cube description:
//!
//! ```text
//!         7 ________ 6           _____6__
//!         /|       /|         7/|       /|
//!       /  |     /  |        /  |     /5 |
//!   4 /_______ /    |      /__4____ /    10
//!    |     |  |5    |     |    11  |     |
//!    |    3|__|_____|2    |     |__|__2__|
//!    |    /   |    /      8   3/   9    /
//!    |  /     |  /        |  /     |  /1
//!    |/_______|/          |/___0___|/
//!   0          1
//!          Vertices              Edges
//! ```

/// The classic marching cubes table
#[rustfmt::skip]
static MARCHING_CUBES_TABLE: [[i32; 16]; 256] = [
/*   0:                          */  [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   1: 0,                       */  [ 0,  8,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   2:    1,                    */  [ 0,  1,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   3: 0, 1,                    */  [ 1,  8,  3,  9,  8,  1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   4:       2,                 */  [ 1,  2, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   5: 0,    2,                 */  [ 0,  8,  3,  1,  2, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   6:    1, 2,                 */  [ 9,  2, 10,  0,  2,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   7: 0, 1, 2,                 */  [ 2,  8,  3,  2, 10,  8, 10,  9,  8, -1, -1, -1, -1, -1, -1, -1],
/*   8:          3,              */  [ 3, 11,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*   9: 0,       3,              */  [ 0, 11,  2,  8, 11,  0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  10:    1,    3,              */  [ 1,  9,  0,  2,  3, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  11: 0, 1,    3,              */  [ 1, 11,  2,  1,  9, 11,  9,  8, 11, -1, -1, -1, -1, -1, -1, -1],
/*  12:       2, 3,              */  [ 3, 10,  1, 11, 10,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  13: 0,    2, 3,              */  [ 0, 10,  1,  0,  8, 10,  8, 11, 10, -1, -1, -1, -1, -1, -1, -1],
/*  14:    1, 2, 3,              */  [ 3,  9,  0,  3, 11,  9, 11, 10,  9, -1, -1, -1, -1, -1, -1, -1],
/*  15: 0, 1, 2, 3,              */  [ 9,  8, 10, 10,  8, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  16:             4,           */  [ 4,  7,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  17: 0,          4,           */  [ 4,  3,  0,  7,  3,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  18:    1,       4,           */  [ 0,  1,  9,  8,  4,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  19: 0, 1,       4,           */  [ 4,  1,  9,  4,  7,  1,  7,  3,  1, -1, -1, -1, -1, -1, -1, -1],
/*  20:       2,    4,           */  [ 1,  2, 10,  8,  4,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  21: 0,    2,    4,           */  [ 3,  4,  7,  3,  0,  4,  1,  2, 10, -1, -1, -1, -1, -1, -1, -1],
/*  22:    1, 2,    4,           */  [ 9,  2, 10,  9,  0,  2,  8,  4,  7, -1, -1, -1, -1, -1, -1, -1],
/*  23: 0, 1, 2,    4,           */  [ 2, 10,  9,  2,  9,  7,  2,  7,  3,  7,  9,  4, -1, -1, -1, -1],
/*  24:          3, 4,           */  [ 8,  4,  7,  3, 11,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  25: 0,       3, 4,           */  [11,  4,  7, 11,  2,  4,  2,  0,  4, -1, -1, -1, -1, -1, -1, -1],
/*  26:    1,    3, 4,           */  [ 9,  0,  1,  8,  4,  7,  2,  3, 11, -1, -1, -1, -1, -1, -1, -1],
/*  27: 0, 1,    3, 4,           */  [ 4,  7, 11,  9,  4, 11,  9, 11,  2,  9,  2,  1, -1, -1, -1, -1],
/*  28:       2, 3, 4,           */  [ 3, 10,  1,  3, 11, 10,  7,  8,  4, -1, -1, -1, -1, -1, -1, -1],
/*  29: 0,    2, 3, 4,           */  [ 1, 11, 10,  1,  4, 11,  1,  0,  4,  7, 11,  4, -1, -1, -1, -1],
/*  30:    1, 2, 3, 4,           */  [ 4,  7,  8,  9,  0, 11,  9, 11, 10, 11,  0,  3, -1, -1, -1, -1],
/*  31: 0, 1, 2, 3, 4,           */  [ 4,  7, 11,  4, 11,  9,  9, 11, 10, -1, -1, -1, -1, -1, -1, -1],
/*  32:                5,        */  [ 9,  5,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  33: 0,             5,        */  [ 9,  5,  4,  0,  8,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  34:    1,          5,        */  [ 0,  5,  4,  1,  5,  0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  35: 0, 1,          5,        */  [ 8,  5,  4,  8,  3,  5,  3,  1,  5, -1, -1, -1, -1, -1, -1, -1],
/*  36:       2,       5,        */  [ 1,  2, 10,  9,  5,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  37: 0,    2,       5,        */  [ 3,  0,  8,  1,  2, 10,  4,  9,  5, -1, -1, -1, -1, -1, -1, -1],
/*  38:    1, 2,       5,        */  [ 5,  2, 10,  5,  4,  2,  4,  0,  2, -1, -1, -1, -1, -1, -1, -1],
/*  39: 0, 1, 2,       5,        */  [ 2, 10,  5,  3,  2,  5,  3,  5,  4,  3,  4,  8, -1, -1, -1, -1],
/*  40:          3,    5,        */  [ 9,  5,  4,  2,  3, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  41: 0,       3,    5,        */  [ 0, 11,  2,  0,  8, 11,  4,  9,  5, -1, -1, -1, -1, -1, -1, -1],
/*  42:    1,    3,    5,        */  [ 0,  5,  4,  0,  1,  5,  2,  3, 11, -1, -1, -1, -1, -1, -1, -1],
/*  43: 0, 1,    3,    5,        */  [ 2,  1,  5,  2,  5,  8,  2,  8, 11,  4,  8,  5, -1, -1, -1, -1],
/*  44:       2, 3,    5,        */  [10,  3, 11, 10,  1,  3,  9,  5,  4, -1, -1, -1, -1, -1, -1, -1],
/*  45: 0,    2, 3,    5,        */  [ 4,  9,  5,  0,  8,  1,  8, 10,  1,  8, 11, 10, -1, -1, -1, -1],
/*  46:    1, 2, 3,    5,        */  [ 5,  4,  0,  5,  0, 11,  5, 11, 10, 11,  0,  3, -1, -1, -1, -1],
/*  47: 0, 1, 2, 3,    5,        */  [ 5,  4,  8,  5,  8, 10, 10,  8, 11, -1, -1, -1, -1, -1, -1, -1],
/*  48:             4, 5,        */  [ 9,  7,  8,  5,  7,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  49: 0,          4, 5,        */  [ 9,  3,  0,  9,  5,  3,  5,  7,  3, -1, -1, -1, -1, -1, -1, -1],
/*  50:    1,       4, 5,        */  [ 0,  7,  8,  0,  1,  7,  1,  5,  7, -1, -1, -1, -1, -1, -1, -1],
/*  51: 0, 1,       4, 5,        */  [ 1,  5,  3,  3,  5,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  52:       2,    4, 5,        */  [ 9,  7,  8,  9,  5,  7, 10,  1,  2, -1, -1, -1, -1, -1, -1, -1],
/*  53: 0,    2,    4, 5,        */  [10,  1,  2,  9,  5,  0,  5,  3,  0,  5,  7,  3, -1, -1, -1, -1],
/*  54:    1, 2,    4, 5,        */  [ 8,  0,  2,  8,  2,  5,  8,  5,  7, 10,  5,  2, -1, -1, -1, -1],
/*  55: 0, 1, 2,    4, 5,        */  [ 2, 10,  5,  2,  5,  3,  3,  5,  7, -1, -1, -1, -1, -1, -1, -1],
/*  56:          3, 4, 5,        */  [ 7,  9,  5,  7,  8,  9,  3, 11,  2, -1, -1, -1, -1, -1, -1, -1],
/*  57: 0,       3, 4, 5,        */  [ 9,  5,  7,  9,  7,  2,  9,  2,  0,  2,  7, 11, -1, -1, -1, -1],
/*  58:    1,    3, 4, 5,        */  [ 2,  3, 11,  0,  1,  8,  1,  7,  8,  1,  5,  7, -1, -1, -1, -1],
/*  59: 0, 1,    3, 4, 5,        */  [11,  2,  1, 11,  1,  7,  7,  1,  5, -1, -1, -1, -1, -1, -1, -1],
/*  60:       2, 3, 4, 5,        */  [ 9,  5,  8,  8,  5,  7, 10,  1,  3, 10,  3, 11, -1, -1, -1, -1],
/*  61: 0,    2, 3, 4, 5,        */  [ 5,  7,  0,  5,  0,  9,  7, 11,  0,  1,  0, 10, 11, 10,  0, -1],
/*  62:    1, 2, 3, 4, 5,        */  [11, 10,  0, 11,  0,  3, 10,  5,  0,  8,  0,  7,  5,  7,  0, -1],
/*  63: 0, 1, 2, 3, 4, 5,        */  [11, 10,  5,  7, 11,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  64:                   6,     */  [10,  6,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  65: 0,                6,     */  [ 0,  8,  3,  5, 10,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  66:    1,             6,     */  [ 9,  0,  1,  5, 10,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  67: 0, 1,             6,     */  [ 1,  8,  3,  1,  9,  8,  5, 10,  6, -1, -1, -1, -1, -1, -1, -1],
/*  68:       2,          6,     */  [ 1,  6,  5,  2,  6,  1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  69: 0,    2,          6,     */  [ 1,  6,  5,  1,  2,  6,  3,  0,  8, -1, -1, -1, -1, -1, -1, -1],
/*  70:    1, 2,          6,     */  [ 9,  6,  5,  9,  0,  6,  0,  2,  6, -1, -1, -1, -1, -1, -1, -1],
/*  71: 0, 1, 2,          6,     */  [ 5,  9,  8,  5,  8,  2,  5,  2,  6,  3,  2,  8, -1, -1, -1, -1],
/*  72:          3,       6,     */  [ 2,  3, 11, 10,  6,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  73: 0,       3,       6,     */  [11,  0,  8, 11,  2,  0, 10,  6,  5, -1, -1, -1, -1, -1, -1, -1],
/*  74:    1,    3,       6,     */  [ 0,  1,  9,  2,  3, 11,  5, 10,  6, -1, -1, -1, -1, -1, -1, -1],
/*  75: 0, 1,    3,       6,     */  [ 5, 10,  6,  1,  9,  2,  9, 11,  2,  9,  8, 11, -1, -1, -1, -1],
/*  76:       2, 3,       6,     */  [ 6,  3, 11,  6,  5,  3,  5,  1,  3, -1, -1, -1, -1, -1, -1, -1],
/*  77: 0,    2, 3,       6,     */  [ 0,  8, 11,  0, 11,  5,  0,  5,  1,  5, 11,  6, -1, -1, -1, -1],
/*  78:    1, 2, 3,       6,     */  [ 3, 11,  6,  0,  3,  6,  0,  6,  5,  0,  5,  9, -1, -1, -1, -1],
/*  79: 0, 1, 2, 3,       6,     */  [ 6,  5,  9,  6,  9, 11, 11,  9,  8, -1, -1, -1, -1, -1, -1, -1],
/*  80:             4,    6,     */  [ 5, 10,  6,  4,  7,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  81: 0,          4,    6,     */  [ 4,  3,  0,  4,  7,  3,  6,  5, 10, -1, -1, -1, -1, -1, -1, -1],
/*  82:    1,       4,    6,     */  [ 1,  9,  0,  5, 10,  6,  8,  4,  7, -1, -1, -1, -1, -1, -1, -1],
/*  83: 0, 1,       4,    6,     */  [10,  6,  5,  1,  9,  7,  1,  7,  3,  7,  9,  4, -1, -1, -1, -1],
/*  84:       2,    4,    6,     */  [ 6,  1,  2,  6,  5,  1,  4,  7,  8, -1, -1, -1, -1, -1, -1, -1],
/*  85: 0,    2,    4,    6,     */  [ 1,  2,  5,  5,  2,  6,  3,  0,  4,  3,  4,  7, -1, -1, -1, -1],
/*  86:    1, 2,    4,    6,     */  [ 8,  4,  7,  9,  0,  5,  0,  6,  5,  0,  2,  6, -1, -1, -1, -1],
/*  87: 0, 1, 2,    4,    6,     */  [ 7,  3,  9,  7,  9,  4,  3,  2,  9,  5,  9,  6,  2,  6,  9, -1],
/*  88:          3, 4,    6,     */  [ 3, 11,  2,  7,  8,  4, 10,  6,  5, -1, -1, -1, -1, -1, -1, -1],
/*  89: 0,       3, 4,    6,     */  [ 5, 10,  6,  4,  7,  2,  4,  2,  0,  2,  7, 11, -1, -1, -1, -1],
/*  90:    1,    3, 4,    6,     */  [ 0,  1,  9,  4,  7,  8,  2,  3, 11,  5, 10,  6, -1, -1, -1, -1],
/*  91: 0, 1,    3, 4,    6,     */  [ 9,  2,  1,  9, 11,  2,  9,  4, 11,  7, 11,  4,  5, 10,  6, -1],
/*  92:       2, 3, 4,    6,     */  [ 8,  4,  7,  3, 11,  5,  3,  5,  1,  5, 11,  6, -1, -1, -1, -1],
/*  93: 0,    2, 3, 4,    6,     */  [ 5,  1, 11,  5, 11,  6,  1,  0, 11,  7, 11,  4,  0,  4, 11, -1],
/*  94:    1, 2, 3, 4,    6,     */  [ 0,  5,  9,  0,  6,  5,  0,  3,  6, 11,  6,  3,  8,  4,  7, -1],
/*  95: 0, 1, 2, 3, 4,    6,     */  [ 6,  5,  9,  6,  9, 11,  4,  7,  9,  7, 11,  9, -1, -1, -1, -1],
/*  96:                5, 6,     */  [10,  4,  9,  6,  4, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/*  97: 0,             5, 6,     */  [ 4, 10,  6,  4,  9, 10,  0,  8,  3, -1, -1, -1, -1, -1, -1, -1],
/*  98:    1,          5, 6,     */  [10,  0,  1, 10,  6,  0,  6,  4,  0, -1, -1, -1, -1, -1, -1, -1],
/*  99: 0, 1,          5, 6,     */  [ 8,  3,  1,  8,  1,  6,  8,  6,  4,  6,  1, 10, -1, -1, -1, -1],
/* 100:       2,       5, 6,     */  [ 1,  4,  9,  1,  2,  4,  2,  6,  4, -1, -1, -1, -1, -1, -1, -1],
/* 101: 0,    2,       5, 6,     */  [ 3,  0,  8,  1,  2,  9,  2,  4,  9,  2,  6,  4, -1, -1, -1, -1],
/* 102:    1, 2,       5, 6,     */  [ 0,  2,  4,  4,  2,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 103: 0, 1, 2,       5, 6,     */  [ 8,  3,  2,  8,  2,  4,  4,  2,  6, -1, -1, -1, -1, -1, -1, -1],
/* 104:          3,    5, 6,     */  [10,  4,  9, 10,  6,  4, 11,  2,  3, -1, -1, -1, -1, -1, -1, -1],
/* 105: 0,       3,    5, 6,     */  [ 0,  8,  2,  2,  8, 11,  4,  9, 10,  4, 10,  6, -1, -1, -1, -1],
/* 106:    1,    3,    5, 6,     */  [ 3, 11,  2,  0,  1,  6,  0,  6,  4,  6,  1, 10, -1, -1, -1, -1],
/* 107: 0, 1,    3,    5, 6,     */  [ 6,  4,  1,  6,  1, 10,  4,  8,  1,  2,  1, 11,  8, 11,  1, -1],
/* 108:       2, 3,    5, 6,     */  [ 9,  6,  4,  9,  3,  6,  9,  1,  3, 11,  6,  3, -1, -1, -1, -1],
/* 109: 0,    2, 3,    5, 6,     */  [ 8, 11,  1,  8,  1,  0, 11,  6,  1,  9,  1,  4,  6,  4,  1, -1],
/* 110:    1, 2, 3,    5, 6,     */  [ 3, 11,  6,  3,  6,  0,  0,  6,  4, -1, -1, -1, -1, -1, -1, -1],
/* 111: 0, 1, 2, 3,    5, 6,     */  [ 6,  4,  8, 11,  6,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 112:             4, 5, 6,     */  [ 7, 10,  6,  7,  8, 10,  8,  9, 10, -1, -1, -1, -1, -1, -1, -1],
/* 113: 0,          4, 5, 6,     */  [ 0,  7,  3,  0, 10,  7,  0,  9, 10,  6,  7, 10, -1, -1, -1, -1],
/* 114:    1,       4, 5, 6,     */  [10,  6,  7,  1, 10,  7,  1,  7,  8,  1,  8,  0, -1, -1, -1, -1],
/* 115: 0, 1,       4, 5, 6,     */  [10,  6,  7, 10,  7,  1,  1,  7,  3, -1, -1, -1, -1, -1, -1, -1],
/* 116:       2,    4, 5, 6,     */  [ 1,  2,  6,  1,  6,  8,  1,  8,  9,  8,  6,  7, -1, -1, -1, -1],
/* 117: 0,    2,    4, 5, 6,     */  [ 2,  6,  9,  2,  9,  1,  6,  7,  9,  0,  9,  3,  7,  3,  9, -1],
/* 118:    1, 2,    4, 5, 6,     */  [ 7,  8,  0,  7,  0,  6,  6,  0,  2, -1, -1, -1, -1, -1, -1, -1],
/* 119: 0, 1, 2,    4, 5, 6,     */  [ 7,  3,  2,  6,  7,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 120:          3, 4, 5, 6,     */  [ 2,  3, 11, 10,  6,  8, 10,  8,  9,  8,  6,  7, -1, -1, -1, -1],
/* 121: 0,       3, 4, 5, 6,     */  [ 2,  0,  7,  2,  7, 11,  0,  9,  7,  6,  7, 10,  9, 10,  7, -1],
/* 122:    1,    3, 4, 5, 6,     */  [ 1,  8,  0,  1,  7,  8,  1, 10,  7,  6,  7, 10,  2,  3, 11, -1],
/* 123: 0, 1,    3, 4, 5, 6,     */  [11,  2,  1, 11,  1,  7, 10,  6,  1,  6,  7,  1, -1, -1, -1, -1],
/* 124:       2, 3, 4, 5, 6,     */  [ 8,  9,  6,  8,  6,  7,  9,  1,  6, 11,  6,  3,  1,  3,  6, -1],
/* 125: 0,    2, 3, 4, 5, 6,     */  [ 0,  9,  1, 11,  6,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 126:    1, 2, 3, 4, 5, 6,     */  [ 7,  8,  0,  7,  0,  6,  3, 11,  0, 11,  6,  0, -1, -1, -1, -1],
/* 127: 0, 1, 2, 3, 4, 5, 6,     */  [ 7, 11,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 128:                      7,  */  [ 7,  6, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 129: 0,                   7,  */  [ 3,  0,  8, 11,  7,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 130:    1,                7,  */  [ 0,  1,  9, 11,  7,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 131: 0, 1,                7,  */  [ 8,  1,  9,  8,  3,  1, 11,  7,  6, -1, -1, -1, -1, -1, -1, -1],
/* 132:       2,             7,  */  [10,  1,  2,  6, 11,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 133: 0,    2,             7,  */  [ 1,  2, 10,  3,  0,  8,  6, 11,  7, -1, -1, -1, -1, -1, -1, -1],
/* 134:    1, 2,             7,  */  [ 2,  9,  0,  2, 10,  9,  6, 11,  7, -1, -1, -1, -1, -1, -1, -1],
/* 135: 0, 1, 2,             7,  */  [ 6, 11,  7,  2, 10,  3, 10,  8,  3, 10,  9,  8, -1, -1, -1, -1],
/* 136:          3,          7,  */  [ 7,  2,  3,  6,  2,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 137: 0,       3,          7,  */  [ 7,  0,  8,  7,  6,  0,  6,  2,  0, -1, -1, -1, -1, -1, -1, -1],
/* 138:    1,    3,          7,  */  [ 2,  7,  6,  2,  3,  7,  0,  1,  9, -1, -1, -1, -1, -1, -1, -1],
/* 139: 0, 1,    3,          7,  */  [ 1,  6,  2,  1,  8,  6,  1,  9,  8,  8,  7,  6, -1, -1, -1, -1],
/* 140:       2, 3,          7,  */  [10,  7,  6, 10,  1,  7,  1,  3,  7, -1, -1, -1, -1, -1, -1, -1],
/* 141: 0,    2, 3,          7,  */  [10,  7,  6,  1,  7, 10,  1,  8,  7,  1,  0,  8, -1, -1, -1, -1],
/* 142:    1, 2, 3,          7,  */  [ 0,  3,  7,  0,  7, 10,  0, 10,  9,  6, 10,  7, -1, -1, -1, -1],
/* 143: 0, 1, 2, 3,          7,  */  [ 7,  6, 10,  7, 10,  8,  8, 10,  9, -1, -1, -1, -1, -1, -1, -1],
/* 144:             4,       7,  */  [ 6,  8,  4, 11,  8,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 145: 0,          4,       7,  */  [ 3,  6, 11,  3,  0,  6,  0,  4,  6, -1, -1, -1, -1, -1, -1, -1],
/* 146:    1,       4,       7,  */  [ 8,  6, 11,  8,  4,  6,  9,  0,  1, -1, -1, -1, -1, -1, -1, -1],
/* 147: 0, 1,       4,       7,  */  [ 9,  4,  6,  9,  6,  3,  9,  3,  1, 11,  3,  6, -1, -1, -1, -1],
/* 148:       2,    4,       7,  */  [ 6,  8,  4,  6, 11,  8,  2, 10,  1, -1, -1, -1, -1, -1, -1, -1],
/* 149: 0,    2,    4,       7,  */  [ 1,  2, 10,  3,  0, 11,  0,  6, 11,  0,  4,  6, -1, -1, -1, -1],
/* 150:    1, 2,    4,       7,  */  [ 4, 11,  8,  4,  6, 11,  0,  2,  9,  2, 10,  9, -1, -1, -1, -1],
/* 151: 0, 1, 2,    4,       7,  */  [10,  9,  3, 10,  3,  2,  9,  4,  3, 11,  3,  6,  4,  6,  3, -1],
/* 152:          3, 4,       7,  */  [ 8,  2,  3,  8,  4,  2,  4,  6,  2, -1, -1, -1, -1, -1, -1, -1],
/* 153: 0,       3, 4,       7,  */  [ 0,  4,  2,  4,  6,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 154:    1,    3, 4,       7,  */  [ 1,  9,  0,  2,  3,  4,  2,  4,  6,  4,  3,  8, -1, -1, -1, -1],
/* 155: 0, 1,    3, 4,       7,  */  [ 1,  9,  4,  1,  4,  2,  2,  4,  6, -1, -1, -1, -1, -1, -1, -1],
/* 156:       2, 3, 4,       7,  */  [ 8,  1,  3,  8,  6,  1,  8,  4,  6,  6, 10,  1, -1, -1, -1, -1],
/* 157: 0,    2, 3, 4,       7,  */  [10,  1,  0, 10,  0,  6,  6,  0,  4, -1, -1, -1, -1, -1, -1, -1],
/* 158:    1, 2, 3, 4,       7,  */  [ 4,  6,  3,  4,  3,  8,  6, 10,  3,  0,  3,  9, 10,  9,  3, -1],
/* 159: 0, 1, 2, 3, 4,       7,  */  [10,  9,  4,  6, 10,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 160:                5,    7,  */  [ 4,  9,  5,  7,  6, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 161: 0,             5,    7,  */  [ 0,  8,  3,  4,  9,  5, 11,  7,  6, -1, -1, -1, -1, -1, -1, -1],
/* 162:    1,          5,    7,  */  [ 5,  0,  1,  5,  4,  0,  7,  6, 11, -1, -1, -1, -1, -1, -1, -1],
/* 163: 0, 1,          5,    7,  */  [11,  7,  6,  8,  3,  4,  3,  5,  4,  3,  1,  5, -1, -1, -1, -1],
/* 164:       2,       5,    7,  */  [ 9,  5,  4, 10,  1,  2,  7,  6, 11, -1, -1, -1, -1, -1, -1, -1],
/* 165: 0,    2,       5,    7,  */  [ 6, 11,  7,  1,  2, 10,  0,  8,  3,  4,  9,  5, -1, -1, -1, -1],
/* 166:    1, 2,       5,    7,  */  [ 7,  6, 11,  5,  4, 10,  4,  2, 10,  4,  0,  2, -1, -1, -1, -1],
/* 167: 0, 1, 2,       5,    7,  */  [ 3,  4,  8,  3,  5,  4,  3,  2,  5, 10,  5,  2, 11,  7,  6, -1],
/* 168:          3,    5,    7,  */  [ 7,  2,  3,  7,  6,  2,  5,  4,  9, -1, -1, -1, -1, -1, -1, -1],
/* 169: 0,       3,    5,    7,  */  [ 9,  5,  4,  0,  8,  6,  0,  6,  2,  6,  8,  7, -1, -1, -1, -1],
/* 170:    1,    3,    5,    7,  */  [ 3,  6,  2,  3,  7,  6,  1,  5,  0,  5,  4,  0, -1, -1, -1, -1],
/* 171: 0, 1,    3,    5,    7,  */  [ 6,  2,  8,  6,  8,  7,  2,  1,  8,  4,  8,  5,  1,  5,  8, -1],
/* 172:       2, 3,    5,    7,  */  [ 9,  5,  4, 10,  1,  6,  1,  7,  6,  1,  3,  7, -1, -1, -1, -1],
/* 173: 0,    2, 3,    5,    7,  */  [ 1,  6, 10,  1,  7,  6,  1,  0,  7,  8,  7,  0,  9,  5,  4, -1],
/* 174:    1, 2, 3,    5,    7,  */  [ 4,  0, 10,  4, 10,  5,  0,  3, 10,  6, 10,  7,  3,  7, 10, -1],
/* 175: 0, 1, 2, 3,    5,    7,  */  [ 7,  6, 10,  7, 10,  8,  5,  4, 10,  4,  8, 10, -1, -1, -1, -1],
/* 176:             4, 5,    7,  */  [ 6,  9,  5,  6, 11,  9, 11,  8,  9, -1, -1, -1, -1, -1, -1, -1],
/* 177: 0,          4, 5,    7,  */  [ 3,  6, 11,  0,  6,  3,  0,  5,  6,  0,  9,  5, -1, -1, -1, -1],
/* 178:    1,       4, 5,    7,  */  [ 0, 11,  8,  0,  5, 11,  0,  1,  5,  5,  6, 11, -1, -1, -1, -1],
/* 179: 0, 1,       4, 5,    7,  */  [ 6, 11,  3,  6,  3,  5,  5,  3,  1, -1, -1, -1, -1, -1, -1, -1],
/* 180:       2,    4, 5,    7,  */  [ 1,  2, 10,  9,  5, 11,  9, 11,  8, 11,  5,  6, -1, -1, -1, -1],
/* 181: 0,    2,    4, 5,    7,  */  [ 0, 11,  3,  0,  6, 11,  0,  9,  6,  5,  6,  9,  1,  2, 10, -1],
/* 182:    1, 2,    4, 5,    7,  */  [11,  8,  5, 11,  5,  6,  8,  0,  5, 10,  5,  2,  0,  2,  5, -1],
/* 183: 0, 1, 2,    4, 5,    7,  */  [ 6, 11,  3,  6,  3,  5,  2, 10,  3, 10,  5,  3, -1, -1, -1, -1],
/* 184:          3, 4, 5,    7,  */  [ 5,  8,  9,  5,  2,  8,  5,  6,  2,  3,  8,  2, -1, -1, -1, -1],
/* 185: 0,       3, 4, 5,    7,  */  [ 9,  5,  6,  9,  6,  0,  0,  6,  2, -1, -1, -1, -1, -1, -1, -1],
/* 186:    1,    3, 4, 5,    7,  */  [ 1,  5,  8,  1,  8,  0,  5,  6,  8,  3,  8,  2,  6,  2,  8, -1],
/* 187: 0, 1,    3, 4, 5,    7,  */  [ 1,  5,  6,  2,  1,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 188:       2, 3, 4, 5,    7,  */  [ 1,  3,  6,  1,  6, 10,  3,  8,  6,  5,  6,  9,  8,  9,  6, -1],
/* 189: 0,    2, 3, 4, 5,    7,  */  [10,  1,  0, 10,  0,  6,  9,  5,  0,  5,  6,  0, -1, -1, -1, -1],
/* 190:    1, 2, 3, 4, 5,    7,  */  [ 0,  3,  8,  5,  6, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 191: 0, 1, 2, 3, 4, 5,    7,  */  [10,  5,  6, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 192:                   6, 7,  */  [11,  5, 10,  7,  5, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 193: 0,                6, 7,  */  [11,  5, 10, 11,  7,  5,  8,  3,  0, -1, -1, -1, -1, -1, -1, -1],
/* 194:    1,             6, 7,  */  [ 5, 11,  7,  5, 10, 11,  1,  9,  0, -1, -1, -1, -1, -1, -1, -1],
/* 195: 0, 1,             6, 7,  */  [10,  7,  5, 10, 11,  7,  9,  8,  1,  8,  3,  1, -1, -1, -1, -1],
/* 196:       2,          6, 7,  */  [11,  1,  2, 11,  7,  1,  7,  5,  1, -1, -1, -1, -1, -1, -1, -1],
/* 197: 0,    2,          6, 7,  */  [ 0,  8,  3,  1,  2,  7,  1,  7,  5,  7,  2, 11, -1, -1, -1, -1],
/* 198:    1, 2,          6, 7,  */  [ 9,  7,  5,  9,  2,  7,  9,  0,  2,  2, 11,  7, -1, -1, -1, -1],
/* 199: 0, 1, 2,          6, 7,  */  [ 7,  5,  2,  7,  2, 11,  5,  9,  2,  3,  2,  8,  9,  8,  2, -1],
/* 200:          3,       6, 7,  */  [ 2,  5, 10,  2,  3,  5,  3,  7,  5, -1, -1, -1, -1, -1, -1, -1],
/* 201: 0,       3,       6, 7,  */  [ 8,  2,  0,  8,  5,  2,  8,  7,  5, 10,  2,  5, -1, -1, -1, -1],
/* 202:    1,    3,       6, 7,  */  [ 9,  0,  1,  5, 10,  3,  5,  3,  7,  3, 10,  2, -1, -1, -1, -1],
/* 203: 0, 1,    3,       6, 7,  */  [ 9,  8,  2,  9,  2,  1,  8,  7,  2, 10,  2,  5,  7,  5,  2, -1],
/* 204:       2, 3,       6, 7,  */  [ 1,  3,  5,  3,  7,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 205: 0,    2, 3,       6, 7,  */  [ 0,  8,  7,  0,  7,  1,  1,  7,  5, -1, -1, -1, -1, -1, -1, -1],
/* 206:    1, 2, 3,       6, 7,  */  [ 9,  0,  3,  9,  3,  5,  5,  3,  7, -1, -1, -1, -1, -1, -1, -1],
/* 207: 0, 1, 2, 3,       6, 7,  */  [ 9,  8,  7,  5,  9,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 208:             4,    6, 7,  */  [ 5,  8,  4,  5, 10,  8, 10, 11,  8, -1, -1, -1, -1, -1, -1, -1],
/* 209: 0,          4,    6, 7,  */  [ 5,  0,  4,  5, 11,  0,  5, 10, 11, 11,  3,  0, -1, -1, -1, -1],
/* 210:    1,       4,    6, 7,  */  [ 0,  1,  9,  8,  4, 10,  8, 10, 11, 10,  4,  5, -1, -1, -1, -1],
/* 211: 0, 1,       4,    6, 7,  */  [10, 11,  4, 10,  4,  5, 11,  3,  4,  9,  4,  1,  3,  1,  4, -1],
/* 212:       2,    4,    6, 7,  */  [ 2,  5,  1,  2,  8,  5,  2, 11,  8,  4,  5,  8, -1, -1, -1, -1],
/* 213: 0,    2,    4,    6, 7,  */  [ 0,  4, 11,  0, 11,  3,  4,  5, 11,  2, 11,  1,  5,  1, 11, -1],
/* 214:    1, 2,    4,    6, 7,  */  [ 0,  2,  5,  0,  5,  9,  2, 11,  5,  4,  5,  8, 11,  8,  5, -1],
/* 215: 0, 1, 2,    4,    6, 7,  */  [ 9,  4,  5,  2, 11,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 216:          3, 4,    6, 7,  */  [ 2,  5, 10,  3,  5,  2,  3,  4,  5,  3,  8,  4, -1, -1, -1, -1],
/* 217: 0,       3, 4,    6, 7,  */  [ 5, 10,  2,  5,  2,  4,  4,  2,  0, -1, -1, -1, -1, -1, -1, -1],
/* 218:    1,    3, 4,    6, 7,  */  [ 3, 10,  2,  3,  5, 10,  3,  8,  5,  4,  5,  8,  0,  1,  9, -1],
/* 219: 0, 1,    3, 4,    6, 7,  */  [ 5, 10,  2,  5,  2,  4,  1,  9,  2,  9,  4,  2, -1, -1, -1, -1],
/* 220:       2, 3, 4,    6, 7,  */  [ 8,  4,  5,  8,  5,  3,  3,  5,  1, -1, -1, -1, -1, -1, -1, -1],
/* 221: 0,    2, 3, 4,    6, 7,  */  [ 0,  4,  5,  1,  0,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 222:    1, 2, 3, 4,    6, 7,  */  [ 8,  4,  5,  8,  5,  3,  9,  0,  5,  0,  3,  5, -1, -1, -1, -1],
/* 223: 0, 1, 2, 3, 4,    6, 7,  */  [ 9,  4,  5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 224:                5, 6, 7,  */  [ 4, 11,  7,  4,  9, 11,  9, 10, 11, -1, -1, -1, -1, -1, -1, -1],
/* 225: 0,             5, 6, 7,  */  [ 0,  8,  3,  4,  9,  7,  9, 11,  7,  9, 10, 11, -1, -1, -1, -1],
/* 226:    1,          5, 6, 7,  */  [ 1, 10, 11,  1, 11,  4,  1,  4,  0,  7,  4, 11, -1, -1, -1, -1],
/* 227: 0, 1,          5, 6, 7,  */  [ 3,  1,  4,  3,  4,  8,  1, 10,  4,  7,  4, 11, 10, 11,  4, -1],
/* 228:       2,       5, 6, 7,  */  [ 4, 11,  7,  9, 11,  4,  9,  2, 11,  9,  1,  2, -1, -1, -1, -1],
/* 229: 0,    2,       5, 6, 7,  */  [ 9,  7,  4,  9, 11,  7,  9,  1, 11,  2, 11,  1,  0,  8,  3, -1],
/* 230:    1, 2,       5, 6, 7,  */  [11,  7,  4, 11,  4,  2,  2,  4,  0, -1, -1, -1, -1, -1, -1, -1],
/* 231: 0, 1, 2,       5, 6, 7,  */  [11,  7,  4, 11,  4,  2,  8,  3,  4,  3,  2,  4, -1, -1, -1, -1],
/* 232:          3,    5, 6, 7,  */  [ 2,  9, 10,  2,  7,  9,  2,  3,  7,  7,  4,  9, -1, -1, -1, -1],
/* 233: 0,       3,    5, 6, 7,  */  [ 9, 10,  7,  9,  7,  4, 10,  2,  7,  8,  7,  0,  2,  0,  7, -1],
/* 234:    1,    3,    5, 6, 7,  */  [ 3,  7, 10,  3, 10,  2,  7,  4, 10,  1, 10,  0,  4,  0, 10, -1],
/* 235: 0, 1,    3,    5, 6, 7,  */  [ 1, 10,  2,  8,  7,  4, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 236:       2, 3,    5, 6, 7,  */  [ 4,  9,  1,  4,  1,  7,  7,  1,  3, -1, -1, -1, -1, -1, -1, -1],
/* 237: 0,    2, 3,    5, 6, 7,  */  [ 4,  9,  1,  4,  1,  7,  0,  8,  1,  8,  7,  1, -1, -1, -1, -1],
/* 238:    1, 2, 3,    5, 6, 7,  */  [ 4,  0,  3,  7,  4,  3, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 239: 0, 1, 2, 3,    5, 6, 7,  */  [ 4,  8,  7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 240:             4, 5, 6, 7,  */  [ 9, 10,  8, 10, 11,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 241: 0,          4, 5, 6, 7,  */  [ 3,  0,  9,  3,  9, 11, 11,  9, 10, -1, -1, -1, -1, -1, -1, -1],
/* 242:    1,       4, 5, 6, 7,  */  [ 0,  1, 10,  0, 10,  8,  8, 10, 11, -1, -1, -1, -1, -1, -1, -1],
/* 243: 0, 1,       4, 5, 6, 7,  */  [ 3,  1, 10, 11,  3, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 244:       2,    4, 5, 6, 7,  */  [ 1,  2, 11,  1, 11,  9,  9, 11,  8, -1, -1, -1, -1, -1, -1, -1],
/* 245: 0,    2,    4, 5, 6, 7,  */  [ 3,  0,  9,  3,  9, 11,  1,  2,  9,  2, 11,  9, -1, -1, -1, -1],
/* 246:    1, 2,    4, 5, 6, 7,  */  [ 0,  2, 11,  8,  0, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 247: 0, 1, 2,    4, 5, 6, 7,  */  [ 3,  2, 11, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 248:          3, 4, 5, 6, 7,  */  [ 2,  3,  8,  2,  8, 10, 10,  8,  9, -1, -1, -1, -1, -1, -1, -1],
/* 249: 0,       3, 4, 5, 6, 7,  */  [ 9, 10,  2,  0,  9,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 250:    1,    3, 4, 5, 6, 7,  */  [ 2,  3,  8,  2,  8, 10,  0,  1,  8,  1, 10,  8, -1, -1, -1, -1],
/* 251: 0, 1,    3, 4, 5, 6, 7,  */  [ 1, 10,  2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 252:       2, 3, 4, 5, 6, 7,  */  [ 1,  3,  8,  9,  1,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 253: 0,    2, 3, 4, 5, 6, 7,  */  [ 0,  9,  1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 254:    1, 2, 3, 4, 5, 6, 7,  */  [ 0,  3,  8, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
/* 255: 0, 1, 2, 3, 4, 5, 6, 7,  */  [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
];

/// Returns a reference into the marching cubes LUT to the case corresponding to the given vertex configuration
pub fn get_marching_cubes_triangulation_raw(vertices_inside: &[bool; 8]) -> &'static [i32; 16] {
    let index = flags_to_index(vertices_inside);
    &MARCHING_CUBES_TABLE[index]
}

/// Returns the marching cubes triangulation corresponding to the given vertex configuration
///
/// In the vertex configuration, a `true` value indicates that the given vertex is inside the
/// iso-surface, i.e. above the iso-surface threshold value. The returned iterator yields
/// at most 5 triangles defined by the indices of the edges of their corner vertices.
pub fn marching_cubes_triangulation_iter(
    vertices_inside: &[bool; 8],
) -> impl Iterator<Item = [i32; 3]> {
    let triangulation = get_marching_cubes_triangulation_raw(vertices_inside);
    (0..5)
        .into_iter()
        .map(move |i| triangulation_to_triangle(triangulation, i))
        .flatten()
}

/// Converts an array of bool representing bits to the corresponding usize, the order of the bits is least to most significant
fn flags_to_index(flags: &[bool; 8]) -> usize {
    let mut index = 0;
    for bit in flags.iter().rev().copied() {
        index = (index << 1) | bit as usize
    }

    index
}

/// Extracts the triangle with the given index from the triangulation
fn triangulation_to_triangle(triangulation: &[i32; 16], triangle_index: usize) -> Option<[i32; 3]> {
    let i = triangle_index;
    if triangulation[3 * i] == -1 {
        None
    } else {
        // Reverse the vertex index order to fix winding order (so that normals point outwards)
        Some([
            triangulation[3 * i + 2],
            triangulation[3 * i + 1],
            triangulation[3 * i + 0],
        ])
    }
}

#[cfg(test)]
#[allow(unused)]
mod test_lut {
    use super::*;

    /// A dumb integer -> bit flags conversion using format!
    fn index_to_flags(index: usize) -> [bool; 8] {
        assert!(index <= 256);

        let b: Vec<char> = format!("{:08b}", index).chars().collect();
        [
            b[7] == '1',
            b[6] == '1',
            b[5] == '1',
            b[4] == '1',
            b[3] == '1',
            b[2] == '1',
            b[1] == '1',
            b[0] == '1',
        ]
    }

    /// Inverts all bools in a flag array
    fn inverse_flags(flags: &[bool; 8]) -> [bool; 8] {
        [
            !flags[0], !flags[1], !flags[2], !flags[3], !flags[4], !flags[5], !flags[6], !flags[7],
        ]
    }

    #[test]
    fn test_flag_conversion_roundtrip() {
        assert_eq!(MARCHING_CUBES_TABLE.len(), 256);

        for i in 0..256 {
            let flags = index_to_flags(i);
            let index = flags_to_index(&flags);

            assert_eq!(i, index);
        }
    }

    #[test]
    fn test_get_marching_cubes_triangulation_raw() {
        assert_eq!(MARCHING_CUBES_TABLE.len(), 256);

        for i in 0..256 {
            assert_eq!(
                MARCHING_CUBES_TABLE[i],
                *get_marching_cubes_triangulation_raw(&index_to_flags(i))
            )
        }
    }

    #[test]
    fn test_get_marching_cubes_triangulation_iter() {
        assert_eq!(MARCHING_CUBES_TABLE.len(), 256);

        for i in 0..256 {
            let flags = index_to_flags(i);
            let raw = get_marching_cubes_triangulation_raw(&flags);

            let mut tri_counter = 0;
            for tri in marching_cubes_triangulation_iter(&flags) {
                let mut vec_raw = raw[3 * tri_counter..3 * tri_counter + 3].to_vec();
                let mut vec_tri = tri.to_vec();

                vec_raw.sort();
                vec_tri.sort();
                assert_eq!(vec_raw, vec_tri);
                tri_counter += 1;
            }

            assert_eq!(
                raw[3 * tri_counter],
                -1,
                "There are more triangles in the raw case then returned by the iterator!"
            )
        }
    }

    #[test]
    fn test_marching_cubes_triangulation_iter() {
        assert!(marching_cubes_triangulation_iter(&[
            false, false, false, false, false, false, false, false
        ])
        .next()
        .is_none(),);

        assert_eq!(
            marching_cubes_triangulation_iter(&[
                true, false, false, false, false, false, false, false
            ])
            .collect::<Vec<_>>(),
            vec![[3, 8, 0]]
        );

        assert_eq!(
            marching_cubes_triangulation_iter(&[
                false, false, true, false, true, false, false, false
            ])
            .collect::<Vec<_>>(),
            vec![[10, 2, 1], [7, 4, 8]]
        );
    }
}