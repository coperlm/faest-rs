# AES 加密算法

## 🔐 AES 在 FAEST 中的角色

FAEST 的核心思想：**证明你知道一个 AES 密钥**，使得：
```
AES(密钥, 输入) = 输出
```

## 📋 三种 AES 变体

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aes {
    Aes128,  // 128位密钥，10轮
    Aes192,  // 192位密钥，12轮
    Aes256,  // 256位密钥，14轮
}
```

## 🔑 密钥和参数

```rust
impl Aes {
    pub const fn get_key_size(self) -> usize {
        match self {
            Aes::Aes128 => 16,  // 字节
            Aes::Aes192 => 24,
            Aes::Aes256 => 32,
        }
    }
    
    pub const fn get_number_rounds(self) -> usize {
        match self {
            Aes::Aes128 => 11,  // 10 + 1
            Aes::Aes192 => 13,
            Aes::Aes256 => 15,
        }
    }
}
```

## 🏗️ AES 结构

### 状态矩阵

AES 将 16 字节数据排列成 4×4 矩阵：

```
[a0  a4  a8  a12]
[a1  a5  a9  a13]
[a2  a6  a10 a14]
[a3  a7  a11 a15]
```

```rust
pub struct AesState(pub [GF2p8; 16]);

impl AesState {
    // 访问状态[行][列]
    fn get(&self, row: usize, col: usize) -> GF2p8 {
        self.0[row + 4 * col]
    }
}
```

### 四个基本操作

#### 1. SubBytes (S-Box)
```rust
// 每个字节通过 S-Box 替换
fn sub_bytes(&mut self) {
    for byte in &mut self.0 {
        *byte = SBOX[byte.0 as usize];
    }
}
```

#### 2. ShiftRows
```rust
// 每行循环左移
// 第0行：不移动
// 第1行：左移1位
// 第2行：左移2位
// 第3行：左移3位
```

#### 3. MixColumns
```rust
// 每列乘以固定矩阵（GF(2^8)上）
[02 03 01 01]
[01 02 03 01]
[01 01 02 03]
[03 01 01 02]
```

#### 4. AddRoundKey
```rust
// XOR 轮密钥
fn add_round_key(&mut self, round_key: &[GF2p8; 16]) {
    for i in 0..16 {
        self.0[i] += round_key[i];  // GF(2^8) 加法
    }
}
```

## 🔄 完整加密流程

```rust
fn encrypt(input: [u8; 16], round_keys: &[[GF2p8; 16]]) -> [u8; 16] {
    let mut state = AesState::from(input);
    
    // 初始轮
    state.add_round_key(&round_keys[0]);
    
    // 中间轮（9轮 for AES-128）
    for round in 1..10 {
        state.sub_bytes();
        state.shift_rows();
        state.mix_columns();
        state.add_round_key(&round_keys[round]);
    }
    
    // 最后一轮（无 MixColumns）
    state.sub_bytes();
    state.shift_rows();
    state.add_round_key(&round_keys[10]);
    
    state.to_bytes()
}
```

## 🔑 密钥扩展

从原始密钥生成轮密钥：

```rust
pub fn expand_key(variant: Aes, key: &[GF2p8]) -> Vec<[GF2p8; 16]> {
    // AES-128: 16字节 → 11个轮密钥（176字节）
    // 使用 S-Box 和轮常量
    ...
}
```

## 🎯 FAEST 的特殊需求

### 扩展见证（Extended Witness）

FAEST 需要记录**所有 S-Box 的输入**：

```rust
pub fn compute_extended_witness(
    key: &[u8],
    input: &[u8; 16],
) -> Option<(Vec<u8>, [u8; 16])> {
    let mut witness = Vec::new();
    
    // 1. 记录密钥
    witness.extend(key);
    
    // 2. 记录密钥扩展中的 S-Box 输入
    let (round_keys, sbox_inputs) = expand_and_collect(...);
    witness.extend(sbox_inputs);
    
    // 3. 记录加密过程中的 S-Box 输入
    let (output, sbox_inputs) = encrypt_and_collect(...);
    witness.extend(sbox_inputs);
    
    Some((witness, output))
}
```

### 为什么需要 S-Box 输入？

```
FAEST 证明：我知道见证 w，使得所有 S-Box 输入都非零

这保证了：
1. 密钥有效（没有全零字节）
2. 加密过程正确
3. 可以安全地进行零知识证明
```

## 💻 代码示例

```rust
use faest::aes::{Aes, AesState};
use faest::gf2psmall::GF2p8;

// 加密一个块
let key: Vec<GF2p8> = vec![...];  // 16个字节
let input = [0u8; 16];

// 扩展密钥
let round_keys = expand_key(Aes::Aes128, &key);

// 加密
let state = AesState::from(input);
let output = state.encrypt(Aes::Aes128, &round_keys);
```

## 📊 性能数据

从你的基准测试结果：

| 变体 | 密钥生成 | 加密块 |
|-----|---------|--------|
| AES-128 | ~30 µs | ~0.1 µs |
| AES-192 | ~53 µs | ~0.12 µs |
| AES-256 | ~57 µs | ~0.14 µs |

## 📍 代码位置

- **定义**：`src/aes.rs` 第1-100行
- **加密**：`src/aes.rs` 第500-700行
- **密钥扩展**：`src/aes.rs` 第300-500行
- **扩展见证**：`src/aes.rs` 第67-100行

## 📚 总结

- AES 是 **FAEST 的核心**
- 密钥 = 私钥，输入输出对 = 公钥
- 扩展见证 = 零知识证明的素材

## 🎓 下一步

理解了 AES，继续学习辅助运算：
→ `08-算术运算.md`
