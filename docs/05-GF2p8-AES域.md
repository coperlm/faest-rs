# GF(2^8) - AES 使用的有限域

## 🎯 从字节到有限域

**GF(2^8)** 是 AES 加密算法使用的有限域，有 **256 个元素**（0-255）。

## 🔢 基本概念

### 元素表示

每个元素可以看作：
1. **整数**：0 到 255
2. **字节**：0x00 到 0xFF
3. **多项式**：$a_7x^7 + a_6x^6 + ... + a_1x + a_0$，其中 $a_i \in \{0,1\}$

```rust
// 代码中的表示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GF2p8(pub u8);

// 示例
let a = GF2p8(0x53);  // 整数：83，二进制：01010011
// 作为多项式：x^6 + x^4 + x + 1
```

## 📐 运算规则

### 加法（XOR）

在 GF(2^8) 中，加法就是按位 XOR：

```rust
GF2p8(0x53) + GF2p8(0xCA) = GF2p8(0x53 ^ 0xCA) = GF2p8(0x99)

// 二进制：
  01010011  (0x53)
⊕ 11001010  (0xCA)
-----------
  10011001  (0x99)
```

### 乘法（模多项式）

乘法比较复杂，需要在模一个**不可约多项式**下进行。

**AES 使用的不可约多项式**：
$$m(x) = x^8 + x^4 + x^3 + x + 1$$

对应：`0x11B`（二进制：100011011）

```rust
// 乘法示例（简化）
GF2p8(0x53) × GF2p8(0xCA) = ?

// 步骤：
1. 多项式乘法（carryless multiplication）
2. 对 m(x) 取模
```

## 💻 代码实现

### 文件：`src/gf2psmall.rs`

```rust
/// GF(2^8) with polynomial x^8 + x^4 + x^3 + x + 1 (the AES field)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, bincode::Encode)]
pub struct GF2p8(pub u8);

impl GF2p8 {
    pub const ORDER: usize = 256;
    pub const LOG_ORDER: u32 = 8;
    
    // 查找表优化（用于乘法）
    const LOG_TABLE: [u8; 256] = [...];
    const ANTI_LOG_TABLE: [Self; 255] = [...];
}
```

### 加法实现

```rust
impl Add for GF2p8 {
    type Output = Self;
    
    fn add(self, other: Self) -> Self {
        Self(self.0 ^ other.0)  // 就是 XOR！
    }
}

impl Sub for GF2p8 {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self {
        Self(self.0 ^ other.0)  // 减法 = 加法
    }
}
```

### 乘法实现（查找表法）

```rust
impl Mul for GF2p8 {
    type Output = Self;
    
    fn mul(self, other: Self) -> Self {
        if self.0 == 0 || other.0 == 0 {
            return Self(0);
        }
        
        // 使用对数表
        let log_a = Self::LOG_TABLE[self.0 as usize];
        let log_b = Self::LOG_TABLE[other.0 as usize];
        let log_result = (log_a as usize + log_b as usize) % 255;
        
        Self::ANTI_LOG_TABLE[log_result]
    }
}
```

## 🎓 查找表原理

### 为什么用查找表？

直接计算多项式乘法很慢，但我们可以利用：

$$a \times b = g^{\log_g(a) + \log_g(b)}$$

其中 $g$ 是 GF(2^8) 的**生成元**。

### 生成元

在 GF(2^8) 中，元素 `0x03` 是一个生成元：

```
g^0 = 1
g^1 = 3
g^2 = 5
g^3 = 15
...
g^254 = 142
g^255 = 1  (循环回到1)
```

### 查找表

```rust
// 对数表：LOG_TABLE[a] = log_g(a)
LOG_TABLE[3] = 1
LOG_TABLE[5] = 2
LOG_TABLE[15] = 3

// 反对数表：ANTI_LOG_TABLE[i] = g^i
ANTI_LOG_TABLE[0] = GF2p8(1)
ANTI_LOG_TABLE[1] = GF2p8(3)
ANTI_LOG_TABLE[2] = GF2p8(5)
```

## 🔍 在 AES 中的应用

### S-Box（替换盒）

AES 的 S-Box 在 GF(2^8) 上定义：

```rust
// S-Box 变换
fn sbox(x: GF2p8) -> GF2p8 {
    if x == GF2p8(0) {
        return GF2p8(0x63);
    }
    
    // 1. 求逆：y = x^(-1)
    let y = x.invert().unwrap();
    
    // 2. 仿射变换
    affine_transform(y)
}
```

### MixColumns

MixColumns 是 GF(2^8) 上的矩阵乘法：

```rust
// 矩阵（固定）
[02 03 01 01]   [a0]
[01 02 03 01] × [a1]
[01 01 02 03]   [a2]
[03 01 01 02]   [a3]

// 02 表示 GF2p8(2)
// 03 表示 GF2p8(3)
// 01 表示 GF2p8(1)
```

## 🧪 实际示例

### 示例1：基本运算

```rust
use faest::gf2psmall::GF2p8;

fn main() {
    let a = GF2p8(0x53);
    let b = GF2p8(0xCA);
    
    // 加法
    let sum = a + b;
    println!("0x53 + 0xCA = 0x{:02X}", sum.0);  // 0x99
    
    // 乘法
    let product = a * b;
    println!("0x53 × 0xCA = 0x{:02X}", product.0);
    
    // 求逆
    if let Some(inv) = a.invert() {
        println!("0x53^(-1) = 0x{:02X}", inv.0);
        // 验证：a × a^(-1) = 1
        assert_eq!(a * inv, GF2p8(1));
    }
}
```

### 示例2：S-Box 计算

```rust
// AES S-Box 的输入输出都是 GF2p8
let input = GF2p8(0x53);
let output = aes_sbox(input);
```

### 示例3：多项式表示

```rust
// 0x53 = 01010011
// = x^6 + x^4 + x + 1

let a = GF2p8(0x53);

// 提取系数
for i in 0..8 {
    let coef = (a.0 >> i) & 1;
    if coef == 1 {
        print!("x^{} + ", i);
    }
}
// 输出: x^0 + x^1 + x^4 + x^6 +
```

## 📊 性质总结

| 性质 | 说明 |
|-----|------|
| **元素个数** | 256 (2^8) |
| **加法单位元** | 0 |
| **乘法单位元** | 1 |
| **加法逆元** | a 的加法逆元是 a 自己 |
| **乘法逆元** | 除了 0，每个元素都有乘法逆元 |
| **生成元** | 0x03 (还有其他) |

## 🔧 其他小有限域

代码还实现了其他小域：

```rust
/// GF(2^7) - 128 个元素
pub struct GF2p7(pub u8);

/// GF(2^9) - 512 个元素
pub struct GF2p9(pub u16);

/// GF(2^10) - 1024 个元素
pub struct GF2p10(pub u16);

/// GF(2^11) - 2048 个元素  
pub struct GF2p11(pub u16);
```

每个都有自己的不可约多项式和查找表。

## 🎯 在 FAEST 中的角色

```rust
// FAEST 可以使用不同的有限域
pub type FaestSigner<F> = ...;

// 示例：使用 GF2p8
let signer = FaestSigner::<GF2p8>::new(secret_key, public_key);

// 内部：
// - AES 密钥用 GF2p8 表示
// - VOLE 计算在 GF2p8 上进行
// - 承诺和证明涉及 GF2p8 向量
```

## 💡 关键代码位置

```rust
// 定义：src/gf2psmall.rs 第28-30行
pub struct GF2p8(pub u8);

// 加法：第300行左右
impl Add for GF2p8 { ... }

// 乘法：第400行左右
impl Mul for GF2p8 { ... }

// 求逆：第600行左右
fn invert(&self) -> CtOption<Self> { ... }

// 查找表：第50-150行
const LOG_TABLE: [u8; 256] = [...];
const ANTI_LOG_TABLE: [Self; 255] = [...];
```

## 🧩 与其他组件的关系

```
GF2p8 ←→ AES
  ↓
VOLE (在 GF2p8 上计算)
  ↓
HomCom (同态承诺)
  ↓
FAEST (签名方案)
```

## 📚 总结

**GF(2^8) 的核心**：
- 256 个元素（一个字节）
- 加法 = XOR（快！）
- 乘法 = 查找表（也快！）
- AES 的数学基础
- FAEST 的计算域

## 🎓 下一步

理解了 GF(2^8)，继续学习：
- **GF(2^128)** - 更大的域 → `06-GF2p128-大域.md`
- **AES 算法** - 如何使用 GF(2^8) → `07-AES算法.md`
