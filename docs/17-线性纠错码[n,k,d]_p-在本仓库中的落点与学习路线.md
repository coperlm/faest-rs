# [n_C,k_C,d_C]_p 线性纠错码：在本仓库中的落点与学习路线

> 目标：你想学的不是“纠错码”这个词，而是论文里 5.2 / 6.1 / 6.2 里**线性码结构**在工程实现里如何被拆成：
>
> - 向量/矩阵的线性运算
> - 随机挑战（随机点）驱动的 Vandermonde（范德蒙德）组合
> - “修正/去随机化（correction/derandomization）”的差值

本仓库（FAEST Rust 实现）里确实很少直接出现 “ECC / error-correcting code / generator matrix” 这些宏观名词，但**线性码的核心动作**（把长对象压缩成短对象、用随机点做一致性校验、用差值强制落入某个子空间）在代码里非常明确。

---

## 0. 先把符号对齐：从 $\mathbb{F}_p$ 到本仓库的 $\mathbb{F}_{2^m}$

你提到的记号是 $[n_C,k_C,d_C]_p$ 线性码（长度 $n_C$、维度 $k_C$、最小距离 $d_C$、字母表是域 $\mathbb{F}_p$）。

但本仓库里用于一致性检查/线性组合的“域”通常是 **$\mathbb{F}_{2^m}$**（小域 GF(2^7)…GF(2^11)），也就是 `SmallGF`。

- 这类域元素定义在 [src/gf2psmall.rs](../src/gf2psmall.rs)（比如 GF(2^8) = AES 域）。
- 在签名参数里切换域大小的证据：测试里按 `F::LOG_ORDER`（7..11）分支校验签名长度：见 [src/fiat_shamir.rs](../src/fiat_shamir.rs#L220-L320)。

因此，你可以把论文里的 “$p$” 泛化理解为 “$q$（域大小）”，本仓库里就是 $q = 2^m$。

---

## 1. 线性码在协议里常见的“三种工程形态”

你关心的 5.2 / 6.1 / 6.2（尤其与线性码相关）在实现里，通常拆成下面三类动作：

1) **“生成矩阵 $G_C$”不以矩阵名出现，而以‘对向量/矩阵做线性组合’出现**

- 典型特征：构造一个 Vandermonde 矩阵，或者对对象按幂次系数求和。

2) **“基矩阵 $T_C$ / 去随机化”不以显式 $T_C^{-1}$ 出现，而以‘发送/保存 correction（差值）’出现**

- 典型特征：`correction_values`、`bit_xor_assign`、或“把随机生成的东西改造成带约束的东西”。

3) **“最小距离 $d_C$ / 健全性放大”不以 $d_C$ 命名出现，而以‘重复次数 τ / num_repetitions’出现**

- 典型特征：`tau`、`num_repetitions`，以及对多轮数据做一致性检查的挑战/响应。

---

## 2. 在本仓库里：你要找的线性码落点在哪里？

### 2.1 你要的“Vandermonde 线性码味道”：`hash_matrix` / `hash_bitvector`

请重点读 [src/arithmetic.rs](../src/arithmetic.rs#L40-L120)。

- `hash_matrix` 的注释明确写了：
  - 计算 $H := R M$，其中 $R$ 是 **(τ × n) 的 Vandermonde 矩阵**，列是随机点 $r$ 的幂次（“successive powers of r”）。
- 这类结构在密码协议里非常常见：
  - 把“长”的矩阵/向量用随机点压缩成“短”的摘要（仍然是线性的）。
  - 直觉上很像 Reed–Solomon/多项式评估风格的线性码生成矩阵（Vandermonde 矩阵就是最常见的生成矩阵形态之一）。

你可以把这里的 $R$ 当作“某个线性码检查/编码用的线性变换”的实例化。

### 2.2 你要的“τ / 重复码 / 健全性放大”：`num_repetitions` 与一致性挑战

请重点读 [src/voleith.rs](../src/voleith.rs#L60-L190)。

这里的关键参数与对象：

- `tau = self.num_repetitions`：重复次数（健全性放大最典型的落点）
- `ell_hat = vole_length + num_repetitions`：扩展长度（协议里经常出现“带冗余/带校验”的扩展长度）
- `u: GF2Vector` 与 `V: Matrix<F>`：VOLE-in-the-Head 的核心输出对象（可以把它们看成被线性约束绑定的随机结构）

更关键的是：**correction（修正）怎么出现的**

- `correction_values: Vec<GF2Vector>` 在 `commit_impl` 里被构造，并被放入 `Commitment`：见 [src/voleith.rs](../src/voleith.rs#L60-L190)。
- 当 `message` 存在时：
  - 代码做了 `msg_correction = msg XOR u_0`（随后把 `u_0` 的前 `vole_length` 位强行替换为 `msg`）。
  - 这是非常典型的“去随机化/修正”：你先随机生成一个对象，再用一个差值让它满足约束。

如果你在论文 5.2 看到类似“通过 $T_C^{-1}$ 把东西投到某个子空间/基上”或“发送 correction matrix” 的叙述，那么在实现里常常就会变形为这种 **XOR 差值 + 覆盖写入** 的结构。

### 2.3 你要的“从 bit 到域元素的线性映射”：`BitMulAccumulate::bitmul_accumulate`

同样在 [src/voleith.rs](../src/voleith.rs#L60-L190) 里，注意这一段模式会出现很多次：

- 对每个 `x`（域元素索引）生成随机比特向量 `r_x_i`
- 把它 XOR 进某个 `u`
- 然后调用 `BitMulAccumulate::bitmul_accumulate(v_i_row, F::from(x), r_x_i_bits)`

直觉解释：

- 这相当于“把 bit-mask 表示的一组选中位置，加上同一个域元素系数”。
- 从线性码角度看，这就是一种**稀疏但结构化的线性编码/组合**：
  - 比特向量提供“选择矩阵”（0/1 系数）
  - `F::from(x)` 提供域上的系数
  - 结果写进 `V` 的某一行

它不一定对应某个你熟悉的经典码（RS/BCH/重复码）“完整编码器”，但它确实是在做“线性空间里的受控随机结构生成”，这正是很多协议把线性码拆解后的工程形态。

---

## 3. 把论文 5.2 / 6.1 / 6.2 的“创新点”映射到本仓库

> 说明：我在仓库里没有看到显式的 `G_C` / `T_C` 变量名；因此这里采用“从实现行为反推论文结构”的方式，把你关心的点落到可读的源码位置。

### 3.1 对应 5.2（你关心的 $G_C$ / $T_C$ / correction）

抓手：`correction_values` 以及“先随机、再修正”的模式。

- `VoleInTheHeadSenderFromVC::commit_impl` 里：
  - 先用 XOF 采样随机比特串，构造 `u` / `V`
  - 再把“让它满足消息约束”的差值放进 `correction_values`
  - 见 [src/voleith.rs](../src/voleith.rs#L60-L190)

学习建议：

- 先把 `message.is_some()` 分支完整跟一遍，把“随机 u_0 → msg_correction → 强制 u_0=msg”理解成一个“投影/修正”。
- 再看 `for i in 1..tau` 分支，把每次 `u_i` 与 `self.u` XOR 后放入 `correction_values` 的意义理解为“跨 repetition 的一致性约束”。

### 3.2 对应 6.1（通用线性码 / 更一般的线性组合）

抓手：Vandermonde 线性压缩（像 RS 风格的生成矩阵）与可选域大小。

- 线性压缩（Vandermonde）：见 [src/arithmetic.rs](../src/arithmetic.rs#L40-L120)
- 可选域大小（GF(2^7)…GF(2^11)）贯穿全协议类型参数 `F: SmallGF`：见 [src/gf2psmall.rs](../src/gf2psmall.rs) 与 [src/fiat_shamir.rs](../src/fiat_shamir.rs#L220-L320)

学习建议：

- 把 `hash_matrix` 视为“从 $\mathbb{F}_q^{n\times m}$ 到 $\mathbb{F}_q^{\tau\times m}$ 的线性映射”。
- 想一想：如果 $M$ 里有两行不同，随机点压缩后碰巧相等的概率是多少？这就是“距离/健全性”的影子。

### 3.3 对应 6.2（重复码 $[\tau,1,\tau]$ 的工程化形态）

抓手：`tau = num_repetitions` 的重复结构。

- 在本仓库里，“重复码”更多以“重复执行 τ 次、然后做一致性检查”的方式出现，而不是以 `G = (1,1,...,1)` 明文出现。
- 你可以把 `num_repetitions` 看成 $n_C=\tau$ 的落点；而 “每一轮都必须同时骗过” 对应距离带来的健全性放大直觉。

源码入口：

- 发送方 commit 的 τ 次结构：见 [src/voleith.rs](../src/voleith.rs#L60-L190)

---

## 4. 推荐学习路线（按“能跑 + 能对照源码 + 能写出你自己的推导”来排）

### 第 1 步：掌握域与线性代数容器

- 读 [src/gf2psmall.rs](../src/gf2psmall.rs)：理解 `SmallGF`、`GF2p8` 等域元素的加乘、以及 `F::LOG_ORDER` 的意义。
- 读 [src/gf2.rs](../src/gf2.rs)：理解 `GF2Vector` 的表示（这决定了“bit-level 的线性约束”如何落地）。

### 第 2 步：把“生成矩阵味道”抓牢

- 精读 [src/arithmetic.rs](../src/arithmetic.rs#L40-L120) 里的 `hash_matrix` 注释与实现。
- 自己写下等式：
  - 给定随机点向量 $r\in \mathbb{F}_q^{\tau}$，构造 Vandermonde $R$，计算 $H=RM$。
  - 这就是“线性码/多项式评估/一致性压缩”的典型工程形态。

### 第 3 步：把“去随机化/修正”抓牢

- 精读 [src/voleith.rs](../src/voleith.rs#L60-L190) 里 `correction_values` 的构造。
- 练习：把 `message.is_some()` 分支翻译成三行数学：
  1) 采样随机 $u_0$
  2) 发送/保存 $corr = msg \oplus u_0$
  3) 令 $u_0 \leftarrow msg$

### 第 4 步：把“码参数怎么选”与安全性直觉对齐

- 把 `tau` 当作最重要的健全性参数：越大越难作弊，但通信/计算也更大。
- 在你自己的笔记里用概率直觉描述：
  - “随机 Vandermonde 压缩”让篡改在随机点下被发现的概率随 τ 增强。

---

## 5. 可运行的最小实验（建议你边读边跑）

1) 跑库测试（确认环境 OK）：

- `cargo test --lib`

2) 跑签名示例（看完整端到端流程）：

- `cargo run --example sign_verify --release`

3) 跑基准（确认 τ 相关代价存在）：

- `cargo bench --bench faest -- keygen --quick`

---

## 6. 你接下来如果要“真正把 [n,k,d]_p 讲清楚”的两条路线

- 路线 A（偏理论）：

  - 你从论文里把“C 是什么码（RS / 一般线性码 / 重复码）”抠出来，明确 $G_C,T_C,d_C$，然后对照本仓库中的 Vandermonde 与 correction，写出逐步等价。
- 路线 B（偏工程）：

  - 你把“本仓库到底用的是哪一种线性码结构”定义为：
    - `hash_matrix` 对应的 Vandermonde 线性映射（像 RS 风格的生成矩阵）
    - `tau` 对应的重复/健全性放大
    - `correction_values` 对应的去随机化
  - 然后你尝试把它抽象成你自己的 `LinearCode` trait（哪怕只写在笔记里，不落代码），明确接口：`encode`、`syndrome/check`、`distance` 的直觉含义。

如果你愿意，我也可以在下一步帮你：

- 把论文里 5.2/6.1/6.2 的关键等式写成“逐行对应本仓库变量”的对照表（你提供论文里的那几个等式/符号定义即可）。
