use crate::common::Size;
use super::ast::Modifier;

use lazy_static::lazy_static;
use std::collections::{HashMap, hash_map};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Matcher {
    // a literal "."
    Dot,

    // a specific literal (basically just an ident)
    Lit(&'static str),

    // immediate literal
    LitInt(u32),

    // float literal
    LitFloat(f32),

    // a random ident
    Ident,

    // a condition code literal
    Cond,

    // immediate
    Imm,

    // Wregisters, XRegisters, etc. match any static register in their family except for SP
    W,
    X,

    // same but addressing the stack pointer instead of the zero register. match any static register in their family except for ZR
    WSP,
    XSP,

    // scalar simd regs
    B,
    H,
    S,
    D,
    Q,

    // vector simd regs
    /// vector register with elements of the specified size. Accepts a lane count of either 64 or 128 total bits
    V(Size),
    /// vector register with elements of the specifized size, with the specified lane count
    VStatic(Size, u8),
    /// vector register with element specifier, with the element of the specified size. The lane count is unchecked.
    VElement(Size),
    /// vector register with element specifier, with the element of the specified size and the element index set to the provided value
    VElementStatic(Size, u8),
    /// vector register with elements of the specified size, with the specified lane count, with an element specifier
    VStaticElement(Size, u8),

    // register list with .0 items, with the elements of size .1
    RegList(u8, Size),
    // register list with .0 items, with the elements of size .1 and a lane count of .2
    RegListStatic(u8, Size, u8),
    // register list with element specifier. It has .0 items with a size of .1
    RegListElement(u8, Size),

    // jump offsets
    Offset,

    // references
    RefBase,
    RefOffset,
    RefPre,
    RefIndex,

    // a single modifier
    LitMod(Modifier),

    // a set of allowed modifiers
    Mod(&'static [Modifier]),

    // possible op mnemnonic end (everything after this point uses the default encoding)
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    // commands that advance the argument pointer
    R(u8), // encode a register, or reference base, into a 5-bit bitfield.
    REven(u8), // same as R, but requires that the register is even.
    RNoZr(u8), // same as R, but does not allow register 31.
    R4(u8), // encode a register in the range 0-15 into a 4-bit bitfield
    RNext, // encode that this register should be the previous register, plus one

    // unsigned immediate encodings
    Ubits(u8, u8), // encodes an unsigned immediate starting at bit .0, .1 bits long
    Uscaled(u8, u8, u8), // encodes an unsigned immediate, starting at bit .0, .1 bits long, shifted .2 bits to the right before encoding
    Ulist(u8, &'static [u16]), // encodes an immediate that can only be a limited amount of options
    Urange(u8, u8, u8), // (loc, min, max) asserts the immediate is below or equal to max, encodes the value of (imm-min)
    Usub(u8, u8, u8), // encodes at .0, .1 bits long, .2 - value. Checks if the value is in the range 1 ..= value
    Unegmod(u8, u8), // encodes at .0, .1 bits long, -value % (1 << .1). Checks if the value is in the range 0 .. value
    Usumdec(u8, u8), // encodes at .0, .1 bits long, the value of the previous arg + the value of the current arg - 1
    Ufields(&'static [u8]), // an immediate, encoded bitwise with the highest bit going into field 0, up to the lowest going into the last bitfield.

    // signed immediate encodings
    Sbits(u8, u8), // encodes a signed immediate starting at bit .0, .1 bits long
    Sscaled(u8, u8, u8), // encodes a signed immediate, starting at bit .0, .1 bits long, shifted .2 bits to the right before encoding

    // bit slice encodings. These don't advance the current argument. Only the slice argument actually encodes anything
    BUbits(u8), // checks if the pointed value fits in the given amount of bits
    BUsum(u8), // checks that the pointed value fits between 1 and (1 << .0) - prev
    BSscaled(u8, u8),
    BUrange(u8, u8), // check if the pointed value is between min/max
    Uslice(u8, u8, u8), // encodes at .0, .1 bits long, the bitslice starting at .2 from the current arg
    Sslice(u8, u8, u8), // encodes at .0, .1 bits long, the bitslice starting at .2 from the current arg

    // special immediate encodings
    Special(u8, SpecialComm),

    // SIMD 128-bit indicator
    Rwidth(u8),

    // Extend/Shift fields
    Rotates(u8), // 2-bits field encoding [LSL, LSR, ASR, ROR]
    ExtendsW(u8), // 3-bits field encoding [UXTB, UXTH, UXTW, UXTX, SXTB, SXTH, SXTW, SXTX]. Additionally, LSL is interpreted as UXTW
    ExtendsX(u8), // 3-bits field encoding [UXTB, UXTH, UXTW, UXTX, SXTB, SXTH, SXTW, SXTX]. Additionally, LSL is interpreted as UXTX

    // Condition encodings.
    /// Normal condition code 4-bit encoding
    Cond(u8),
    /// Condition 4-bit encoding, but the last bit is inverted. No AL/NV allowed
    CondInv(u8),

    // Mapping of literal -> bitvalue
    LitList(u8, &'static str),

    // Offsets
    Offset(Relocation),

    // special commands
    A, // advances the argument pointer, only needed to skip over an argument.
    C, // moves the argument pointer back.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum SpecialComm {
    INVERTED_WIDE_IMMEDIATE_W,
    INVERTED_WIDE_IMMEDIATE_X,
    WIDE_IMMEDIATE_W,
    WIDE_IMMEDIATE_X,
    STRETCHED_IMMEDIATE,
    LOGICAL_IMMEDIATE_W,
    LOGICAL_IMMEDIATE_X,
    FLOAT_IMMEDIATE,
    SPLIT_FLOAT_IMMEDIATE,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relocation {
    // b, bl 26 bits, dword aligned
    B = 0,
    // b.cond, cbnz, cbz, ldr, ldrsw, prfm: 19 bits, dword aligned
    BCOND = 1,
    // adr split 21 bit, byte aligned
    ADR = 2,
    // adrp split 21 bit, 4096-byte aligned
    ADRP = 3,
    // tbnz, tbz: 14 bits, dword aligned
    TBZ = 4,
    // 8-bit literal
    LITERAL8 = 5,
    // 16-bit literal
    LITERAL16 = 6,
    // 32-bit literal
    LITERAL32 = 8,
    // 64-bit literal
    LITERAL64 = 12,
}

impl Relocation {
    pub fn to_id(&self) -> u8 {
        *self as u8
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Opdata {
    /// The base template for the encoding.
    pub base: u32,
    /// A set of matchers capable of matching the instruction encoding that this instruction represents.
    pub matchers: &'static [Matcher],
    /// A sequence of encoder commands that check the matched instruction on validity and whose output gets orred together with the original template at runtime.
    pub commands: &'static [Command]
}

macro_rules! SingleOp {
    ( $base:expr, [ $( $matcher:expr ),* ], [ $( $command:expr ),* ] ) => {
        {
            const MATCHERS: &'static [Matcher] = {
                #[allow(unused_imports)]
                use self::Matcher::*;
                &[ $(
                    $matcher
                ),* ]
            };
            const COMMANDS: &'static [Command] = {
                #[allow(unused_imports)]
                use self::Command::*;
                &[ $(
                    $command
                ),* ]
            };
            Opdata {
                base: $base,
                matchers: MATCHERS,
                commands: COMMANDS,
            }
        }
    }
}

macro_rules! Ops {
    ( $( $name:tt = [ $( $base:tt = [ $( $matcher:expr ),* ] => [ $( $command:expr ),* ] ; )+ ] )* ) => {
        [ $(
            (
                $name,
                &[ $(
                    SingleOp!( $base, [ $( $matcher ),* ], [ $( $command ),* ] )
                ),+ ] as &[_]
            )
        ),* ]
    }
}

pub fn get_mnemonic_data(name: &str) -> Option<&'static [Opdata]> {
    OPMAP.get(&name).cloned()
}

#[allow(dead_code)]
pub fn mnemnonics() -> hash_map::Keys<'static, &'static str, &'static [Opdata]> {
    OPMAP.keys()
}

lazy_static! {
    static ref OPMAP: HashMap<&'static str, &'static [Opdata]> = {
        use super::ast::Modifier::*;
        use crate::common::Size::*;
        use self::SpecialComm::*;
        use self::Relocation::*;

        const EXTENDS: &'static [super::ast::Modifier] = &[UXTB, UXTH, UXTW, UXTX, SXTB, SXTH, SXTW, SXTX, LSL];
        const EXTENDS_W: &'static [super::ast::Modifier] = &[UXTB, UXTH, UXTW, SXTB, SXTH, SXTW];
        const EXTENDS_X: &'static [super::ast::Modifier] = &[UXTX, SXTX, LSL];
        const SHIFTS: &'static [super::ast::Modifier] = &[LSL, LSR, ASR];
        const ROTATES: &'static [super::ast::Modifier] = &[LSL, LSR, ASR, ROR];

        static MAP: &[(&str, &[Opdata])] = &include!("opmap.rs");
        MAP.iter().cloned().collect()
    };

    /// A map of existing condition codes and their normal encoding
    pub static ref COND_MAP: HashMap<&'static str, u8> = {
        static MAP: &[(&str, u8)] = &[
            ("eq", 0),
            ("ne", 1),
            ("cs", 2),
            ("hs", 2),
            ("cc", 3),
            ("lo", 3),
            ("mi", 4),
            ("pl", 5),
            ("vs", 6),
            ("vc", 7),
            ("hi", 8),
            ("ls", 9),
            ("ge", 10),
            ("lt", 11),
            ("gt", 12),
            ("le", 13),
            ("al", 14),
            ("nv", 15),
        ];
        MAP.iter().cloned().collect()
    };

    // special ident maps
    pub static ref SPECIAL_IDENT_MAP: HashMap<&'static str, HashMap<&'static str, u32>> = {
        let mut mapmap = HashMap::new();
        mapmap.insert("AT_OPS", {
            static MAP: &[(&str, u32)] = &[
                ("s1e1r",  0b00001111000000),
                ("s1e1w",  0b00001111000001),
                ("s1e0r",  0b00001111000010),
                ("s1e0w",  0b00001111000011),
                ("s1e2r",  0b10001111000000),
                ("s1e2w",  0b10001111000001),
                ("s12e1r", 0b10001111000100),
                ("s12e1w", 0b10001111000101),
                ("s12e0r", 0b10001111000110),
                ("s12e0w", 0b10001111000111),
                ("s1e3r",  0b11001111000000),
                ("s1e3w",  0b11001111000001),
                ("s1e1rp", 0b00001111001000),
                ("s1e1wp", 0b00001111001001),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap.insert("IC_OPS", {
            static MAP: &[(&str, u32)] = &[
                ("ialluis", 0b00001110001000),
                ("iallu",   0b00001110101000),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap.insert("DC_OPS", {
            static MAP: &[(&str, u32)] = &[
                ("ivac",  0b00001110110001),
                ("isw",   0b00001110110010),
                ("csw",   0b00001111010010),
                ("cisw",  0b00001111110010),
                ("zva",   0b01101110100001),
                ("cvac",  0b01101111010001),
                ("cvau",  0b01101111011001),
                ("civac", 0b01101111110001),
                ("cvap",  0b01101111100001),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap.insert("BARRIER_OPS", {
            static MAP: &[(&str, u32)] = &[
                ("sy",    0b1111),
                ("st",    0b1110),
                ("ld",    0b1101),
                ("ish",   0b1011),
                ("ishst", 0b1010),
                ("ishld", 0b1001),
                ("nsh",   0b0111),
                ("nshst", 0b0110),
                ("nshld", 0b0101),
                ("osh",   0b0011),
                ("oshst", 0b0010),
                ("oshld", 0b0001),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap.insert("MSR_IMM_OPS", {
            static MAP: &[(&str, u32)] = &[
                ("spsel",   0b00001000000101),
                ("daifset", 0b01101000000110),
                ("daifclr", 0b01101000000111),
                ("uao",     0b00001000000011),
                ("pan",     0b00001000000100),
                ("dit",     0b01101000000010),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap.insert("CONTROL_REGS", {
            static MAP: &[(&str, u32)] = &[
                ("c0",  0),
                ("c1",  1),
                ("c2",  2),
                ("c3",  3),
                ("c4",  4),
                ("c5",  5),
                ("c6",  6),
                ("c7",  7),
                ("c8",  8),
                ("c9",  9),
                ("c10", 10),
                ("c11", 11),
                ("c12", 12),
                ("c13", 13),
                ("c14", 14),
                ("c15", 15),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap.insert("TLBI_OPS", {
            static MAP: &[(&str, u32)] = &[
                ("vmalle1is",    0b00010000011000),
                ("vae1is",       0b00010000011001),
                ("aside1is",     0b00010000011010),
                ("vaae1is",      0b00010000011011),
                ("vale1is",      0b00010000011101),
                ("vaale1is",     0b00010000011111),
                ("vmalle1",      0b00010000111000),
                ("vae1",         0b00010000111001),
                ("aside1",       0b00010000111010),
                ("vaae1",        0b00010000111011),
                ("vale1",        0b00010000111101),
                ("vaale1",       0b00010000111111),
                ("ipas2e1is",    0b10010000000001),
                ("ipas2le1is",   0b10010000000101),
                ("alle2is",      0b10010000011000),
                ("vae2is",       0b10010000011001),
                ("alle1is",      0b10010000011100),
                ("vale2is",      0b10010000011101),
                ("vmalls12e1is", 0b10010000011110),
                ("ipas2e1",      0b10010000100001),
                ("ipas2le1",     0b10010000100101),
                ("alle2",        0b10010000111000),
                ("vae2",         0b10010000111001),
                ("alle1",        0b10010000111100),
                ("vale2",        0b10010000111101),
                ("vmalls12e1",   0b10010000111110),
                ("alle3is",      0b11010000011000),
                ("vae3is",       0b11010000011001),
                ("vale3is",      0b11010000011101),
                ("alle3",        0b11010000111000),
                ("vae3",         0b11010000111001),
                ("vale3",        0b11010000111101),
                ("vmalle1os",    0b00010000001000),
                ("vae1os",       0b00010000001001),
                ("aside1os",     0b00010000001010),
                ("vaae1os",      0b00010000001011),
                ("vale1os",      0b00010000001101),
                ("vaale1os",     0b00010000001111),
                ("rvae1is",      0b00010000010001),
                ("rvaae1is",     0b00010000010011),
                ("rvale1is",     0b00010000010101),
                ("rvaale1is",    0b00010000010111),
                ("rvae1os",      0b00010000101001),
                ("rvaae1os",     0b00010000101011),
                ("rvale1os",     0b00010000101101),
                ("rvaale1os",    0b00010000101111),
                ("rvae1",        0b00010000110001),
                ("rvaae1",       0b00010000110011),
                ("rvale1",       0b00010000110101),
                ("rvaale1",      0b00010000110111),
                ("ripas2e1is",   0b10010000000010),
                ("ripas2le1is",  0b10010000000110),
                ("alle2os",      0b10010000001000),
                ("vae2os",       0b10010000001001),
                ("alle1os",      0b10010000001100),
                ("vale2os",      0b10010000001101),
                ("vmalls12e1os", 0b10010000001110),
                ("rvae2is",      0b10010000010001),
                ("rvale2is",     0b10010000010101),
                ("ipas2e1os",    0b10010000100000),
                ("ripas2e1",     0b10010000100010),
                ("ripas2e1os",   0b10010000100011),
                ("ipas2le1os",   0b10010000100100),
                ("ripas2le1",    0b10010000100110),
                ("ripas2le1os",  0b10010000100111),
                ("rvae2os",      0b10010000101001),
                ("rvale2os",     0b10010000101101),
                ("rvae2",        0b10010000110001),
                ("rvale2",       0b10010000110101),
                ("alle3os",      0b11010000001000),
                ("vae3os",       0b11010000001001),
                ("vale3os",      0b11010000001101),
                ("rvae3is",      0b11010000010001),
                ("rvale3is",     0b11010000010101),
                ("rvae3os",      0b11010000101001),
                ("rvale3os",     0b11010000101101),
                ("rvae3",        0b11010000110001),
                ("rvale3",       0b11010000110101),
            ];
            MAP.iter().cloned().collect()
        });
        mapmap
    };
}
