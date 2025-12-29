# GGM 向量承诺（GgmVecCom）源码走读：如何构建与如何使用

这份文档专门解释本仓库里 **GGM 向量承诺**（Vector Commitment）的“真实实现细节”，以源码为主线：

- `src/veccom.rs`：`GgmVecCom` 的 `commit/decommit/verify/recompute_commitment`
- `src/primitives.rs`：PRG（`Aes128CtrLdPrg`）与叶子扩展器（`Blake3LE`）
- `src/common.rs`：`prefix_decompose`（路径前缀分解）
- `src/voleith.rs`：VOLE-in-the-Head 如何调用向量承诺

> 你可以把这里的 `GgmVecCom` 理解成：
> 1) 用 PRG 从根种子扩展出一整棵二叉树的“叶子种子”；
> 2) 每个叶子种子再扩展成 `(com_j, xof_j)`；
> 3) 把所有 `com_j` 按顺序拼起来做一次哈希，得到整体承诺 `Commitment`。

---

## 0. 本仓库里的接口：`VecCom`

`src/veccom.rs` 定义了 trait：

```rust
pub trait VecCom {
    type Commitment: Clone + Eq + AsRef<[u8]>;
    type DecommitmentKey: Copy;
    type Decommitment: Clone;
    type XofReader: XofReader;

    fn commit(log_num_messages: u32)
        -> (Self::Commitment, Self::DecommitmentKey, Vec<Self::XofReader>);

    fn decommit(log_num_messages: u32, decommitment_key: Self::DecommitmentKey, index: usize)
        -> Self::Decommitment;

    fn verify(log_num_messages: u32, commitment: &Self::Commitment,
              decommitment: &Self::Decommitment, index: usize)
        -> Option<Vec<Self::XofReader>>;

    fn recompute_commitment(log_num_messages: u32, decommitment: &Self::Decommitment, index: usize)
        -> (Self::Commitment, Vec<Self::XofReader>);
}
```

理解这 4 个函数的职责：

- `commit(log_n)`：一次性“承诺”一个长度为 $2^{log\_n}$ 的向量（注意：本实现并不直接把“消息向量”作为输入，而是输出每个位置的 `XofReader`，让上层协议从中读取伪随机流作为消息/随机性来源）。
- `decommit(log_n, key, index)`：生成打开 **某一个位置** `index` 的去承诺证明（路径信息）。
- `recompute_commitment(log_n, decommitment, index)`：验证者根据去承诺信息，重建“整棵树”的所有叶子承诺（其中 `index` 那个叶子承诺值由去承诺直接提供），并重新计算整体承诺哈希。
- `verify(...)`：只是把 `recompute_commitment` 的结果与外部给定的 `commitment` 做相等检查，相等则返回可用的 `xofs`。

---

## 1. GGM 树如何表示：数组索引（Binary Heap 风格）

本实现的树不是用指针结构，而是用一个 `Vec<B>` 存所有“节点种子”，并用索引来表达父子关系：

- 节点 `j` 的两个孩子是：`2*j` 和 `2*j + 1`
- 根在 `seeds[0]`

你会在 `commit` 里看到：

```rust
let mut seeds = vec![B::default(); num_messages];
seeds[0] = decommitment_key;
for level_l in 0..tree_height {
    for j in (0..1 << level_l).rev() {
        let (k0, k1) = Prg::expand(seeds[j]);
        seeds[2 * j] = k0;
        seeds[2 * j + 1] = k1;
    }
}
```

几点要注意：

1) `num_messages = 1 << log_num_messages`，也就是叶子数量。

2) 这个 `seeds` 的长度只有 `num_messages`，不是 `2*num_messages`。这意味着：

- 这里的“节点编号”并不是传统二叉堆完整树那套（完整树一般需要 `2n` 空间）。
- 它更像是把“扩展过程的中间层”复用到同一个数组里，最终留下长度为 `num_messages` 的“叶子种子”。

3) 内层循环 `for j in (0..1<<level_l).rev()` 选择倒序是为了避免覆盖依赖（先把父节点扩展成孩子，写回到更大的索引位置）。

> 你可以把这个过程当成“原地扩展”：每一层把前一层的种子扩展成下一层，最终 `seeds[0..num_messages)` 变成所有叶子种子。

---

## 2. 路径如何计算：`prefix_decompose`

`src/common.rs`：

```rust
pub fn prefix_decompose(x: usize, n_bits: u32) -> Vec<usize> {
    (0..n_bits).rev().map(|i| x >> i).collect()
}
```

它返回的是：从最高位到最低位逐步右移得到的前缀序列。

举例：`x = b3b2b1b0`（4 位），返回：

- `x>>3`（1 位前缀）
- `x>>2`（2 位前缀）
- `x>>1`（3 位前缀）
- `x>>0`（4 位前缀，也就是 x 本身）

在 `decommit` 和 `recompute_commitment` 里：

```rust
let index_prefixes = prefix_decompose(index, log_num_messages);
```

然后用 `index_prefixes[level]` 表示从根走到某层时的“节点编号前缀”。

---

## 3. `commit`：如何生成承诺

### 3.1 输出是什么？

`commit(log_n)` 输出 3 个东西：

1) `commitment: Commitment<H>`：整体承诺哈希（digest 输出包了一层 `Commitment` 结构体）
2) `decommitment_key: B`：根种子（之后打开某个位置必须用到）
3) `xofs: Vec<LE::XofR>`：每个叶子对应的 XOF reader（上层协议从里面读出“该叶子承诺的消息流/随机流”）

### 3.2 根种子与 PRG

在本仓库里：

- `B` 通常是 `Block128([u8;16])`
- `Prg` 通常是 `Aes128CtrLdPrg`

`src/primitives.rs`：

```rust
impl LengthDoubling for Aes128CtrLdPrg {
  type Block = Block128;
  fn expand(seed: Block128) -> (Block128, Block128) {
      // 把 seed 当作 AES key
      // 分别加密两个固定块 [..,0] 和 [..,1] 得到两个孩子
  }
}
```

直观理解：

- 父种子是 128 位
- 通过 AES(key=父种子) 加密两块常量，得到左右孩子种子

### 3.3 叶子扩展（LeafExpander）

树扩展到叶子种子后，并不直接把叶子种子当作“承诺值”。还要再做一步“叶子扩展”：

`src/primitives.rs`：

```rust
impl<Com> LeafExpander for Blake3LE<Com> {
  type XofR = blake3::OutputReader;
  fn expand_leaf(leaf: impl AsRef<[u8]>) -> (Com, OutputReader) {
      let mut hasher = blake3::Hasher::new();
      hasher.update(leaf.as_ref());
      let mut xof = hasher.finalize_xof();
      let mut com: Com = Default::default();
      xof.fill(com.as_mut());
      (com, xof)
  }
}
```

这一步产生：

- `com_j`：固定长度输出（`Com` 通常也是 `Block128`），用于“整体承诺哈希”的输入
- `xof_j`：可扩展输出流（XOF reader），上层协议能从中读无限伪随机字节

### 3.4 整体承诺哈希怎么做

`commit` 的核心：

```rust
let mut hasher = H::new();
seeds.into_iter().for_each(|s_j| {
    let (com_j, xof_j) = LE::expand_leaf(s_j);
    hasher.update(com_j);
    xofs.push(xof_j);
});
let commitment = hasher.finalize();
```

也就是：

$$
C = H(com_0 \|\| com_1 \|\| \dots \|\| com_{n-1})
$$

顺序非常重要：验证者重算也必须是同样顺序。

---

## 4. `decommit`：如何打开某个叶子

`decommit(log_n, decommitment_key, index)` 返回：

```rust
type Decommitment = (Vec<B>, B);
```

也就是：

- `Vec<B>`：路径上每一层的“兄弟种子”（sibling seeds）
- `B`：被打开叶子的 `com_i`

### 4.1 直觉

验证者要重建整棵树的所有 `com_j`，但不会知道 `index` 那条路径上的所有种子。

去承诺的做法是：

- 对于 `index` 路径上每一层，把“另一边子树的根种子”给验证者。
- 验证者拿到这些种子，就能用 PRG + leaf expander 把“整棵树除了 index 那条路径缺失的那部分”都补出来。

而 `index` 这个叶子的 `com_i` 直接给（而不是给种子），让验证者能正确重算整体哈希但又不暴露该叶子对应的 `xof_i`（注意：本实现仍然会返回一个 `xof_i` 占位 reader，但注释说它不该被使用）。

### 4.2 源码如何做

`src/veccom.rs`：

```rust
let index_prefixes = prefix_decompose(index, log_num_messages);

let mut seeds = vec![B::default(); num_messages];
let mut decommitment = Vec::with_capacity(tree_height);
seeds[0] = decommitment_key;
(seeds[0], seeds[1]) = Prg::expand(seeds[0]);
for level_l in 1..tree_height {
    decommitment.push(seeds[index_prefixes[level_l - 1] ^ 1]);
    let i = index_prefixes[level_l - 1];
    let (k0, k1) = Prg::expand(seeds[i]);
    seeds[2 * i] = k0;
    seeds[2 * i + 1] = k1;
}

decommitment.push(seeds[index_prefixes[tree_height - 1] ^ 1]);
let (com_i, _) = LE::expand_leaf(seeds[index]);
(decommitment, com_i)
```

其中 `^ 1` 就是“兄弟节点”：

- 如果当前是偶数索引（左孩子），`i ^ 1 = i+1`
- 如果当前是奇数索引（右孩子），`i ^ 1 = i-1`

---

## 5. `recompute_commitment`：验证者如何重算整体承诺

验证者拿到 `(siblings, com_i)` 后：

1) 把每层 sibling seed 填到正确位置
2) 用 PRG 把其他需要的种子扩展出来
3) 对每个种子做 leaf expander 得到 `com_j`
4) 用同样的 `H` 哈希所有 `com_j`，但对 `j==index` 时使用去承诺给的 `com_i`

源码关键点：

```rust
for level_l in 1..tree_height {
    let i = index_prefixes[level_l - 1];
    seeds[i ^ 1] = decommitment[level_l - 1];

    for j in (0..1 << level_l).rev() {
        if j == i { continue; }
        let (k0, k1) = Prg::expand(seeds[j]);
        seeds[2 * j] = k0;
        seeds[2 * j + 1] = k1;
    }
}

seeds[index ^ 1] = decommitment[tree_height - 1];

let mut hasher = H::new();
seeds.into_iter().enumerate().for_each(|(j, s_j)| {
    let (com_j, xof_j) = LE::expand_leaf(s_j);
    if j == index {
        hasher.update(com_i);
        xofs.push(xof_j); // 占位
    } else {
        hasher.update(com_j);
        xofs.push(xof_j);
    }
});
```

最后 `verify` 只是检查：

```rust
if recomputed_commitment == *commitment { Some(xofs) } else { None }
```

---

## 6. 这个 VC 在 VOLE-in-the-Head 里怎么用

这里是最关键的“使用方式”，它解释了：为什么 `VecCom::commit` 不直接承诺一个“消息向量”，而是输出一堆 `xof`。

### 6.1 在 Sender：`VC::commit(log_q)` 产生 q 个叶子流

`src/voleith.rs`（节选）：

- 每次 iteration 都会：
  - 调用 `VC::commit(log_q)`
  - 把 `commitment`（整棵树的哈希）喂进更外层的 hasher
  - 用 `xofs[x]` 读出一段伪随机比特串 `r_{x,i}`

```rust
let (commitment, decommitment_key, mut xofs) = VC::commit(log_q);
self.decommitment_keys.push(decommitment_key);
hasher.update(commitment.as_ref());

// x = 0 的叶子流读出 u
xofs[0].read(u_0.bits.as_raw_mut_slice());

// x = 1..q-1 的叶子流读出 r_x
for (x, xof_x) in xofs.iter_mut().enumerate().skip(1) {
    xof_x.read(r_x_0.as_raw_mut_slice());
    ...
}
```

直觉：

- 这里把“每个叶子”当成一个确定性随机源（XOF），从中取出 VOLE 所需的随机性。
- `log_q` 来自小域 `F` 的阶：`F::LOG_ORDER`，所以这棵 GGM 树的叶子数是 `q = F::ORDER`。

### 6.2 在 Sender：`VC::decommit(log_q, key_i, Delta_i)`

最终挑战阶段（这里的 `Delta_i` 是某个小域元素）：

```rust
let decommitments = Deltas
  .iter()
  .zip(self.decommitment_keys.iter())
  .map(|(Delta_i, &decommitment_key)| {
      VC::decommit(log_q, decommitment_key, (*Delta_i).into())
  })
  .collect();
```

注意：这里把 `Delta_i` 当作“要打开的叶子 index”。

也就是说：

- 每次 VC 承诺了一棵树（q 个叶子）
- 之后只打开其中一个叶子（索引为 `Delta_i`）

### 6.3 在 Receiver：用 `VC::recompute_commitment` 重算并检查哈希

`src/voleith.rs` 的 `receive_decommitment`：

```rust
let (recomputed_commitment, mut xofs) =
    VC::recompute_commitment(log_q, &decommitments[i], Delta_i.into());
hasher.update(recomputed_commitment.as_ref());
...

if hasher.finalize() != self.vc_commitment_hash { return false; }
```

Receiver 通过重算每轮的 VC 承诺哈希并汇总，最终检查是否与 Sender 最初发的 `vc_commitment_hash` 一致。

一旦一致，Receiver 就获得了一个 `xofs` 列表（每个叶子的 XOF reader），从而能重建 VOLE 里的各种矩阵/向量。

---

## 7. 安全性直觉（结合本实现）

在这份实现里，你可以这样理解安全性来源：

- **伪随机性**：
  - `Prg::expand`（AES-based）保证从根种子展开的叶子种子看起来随机。
  - `LeafExpander`（BLAKE3 XOF）把叶子种子变成承诺块 `com_j` 与可扩展随机流 `xof_j`。

- **绑定性（Binding）**：
  - 整体承诺是对所有 `com_j` 做的哈希 `H`。
  - 去承诺中只给出路径上的 sibling seeds + `com_i`。
  - 若想在保持 `commitment` 不变的情况下改动某个叶子的 `com_i`，需要找到哈希碰撞或破坏路径一致性（在安全假设下不可行）。

- **选择性打开**：
  - `decommit` 只透露与路径相关的 sibling seeds，理论上不足以还原其他叶子的种子与 `xof` 流。

---

## 8. 你可以自己做的两个小实验（帮助真正理解）

### 实验 A：把 `log_num_messages` 设小一点手算路径

用 `log_num_messages = 3`（8 个叶子），随便选 `index = 5`（二进制 101）。

- 用 `prefix_decompose(5, 3)` 看看每层 `i` 是多少
- 看每层 `i ^ 1` 是谁
- 对照 `decommit` 输出的 `Vec<B>` 顺序

### 实验 B：在 `src/veccom.rs` 的测试里打印 `com_j`

测试 `test_veccom_correctness` 里：

- Sender 的 `xofs_sender[i]` 读出 `messages_sender[i]`
- Receiver 通过 `verify` 复原出 `xofs_receiver`，除 `index` 外应当都和 Sender 一致

这能直观看到“只打开 1 个叶子，但能让接收方重构所有其他叶子流”的效果。

---

## 9. 小结：一句话记住本实现

- `GgmVecCom::commit(log_q)`：生成 **q 个叶子随机流**，并用所有叶子的 `com` 哈希成一个 `commitment`。
- `GgmVecCom::decommit(..., index)`：给出打开 `index` 所需的“路径兄弟种子 + com_index”。
- `recompute_commitment`：验证者用去承诺信息补全整棵树并重算哈希。
- 在 VOLE-in-the-Head 中：`index` 不是普通索引，而是 `Delta_i`（小域元素）被转成 `usize` 后作为打开的位置。
