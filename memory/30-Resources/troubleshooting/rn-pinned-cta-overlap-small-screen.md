---
title: RN 贴底 CTA 外的固定高度内容在小屏溢出压住按钮（View 不裁剪）
date: 2026-09-22
tags: [troubleshooting, react-native, layout, small-screen, sitin-rn, luka]
---

# RN 贴底 CTA 外的固定高度内容在小屏溢出压住按钮（View 不裁剪）

来源：sitin-rn `apps/luka` 注册第 3 步「How do you identify?」（`src/app/onboarding/gender.tsx`），2026-09-22 用户发 iPhone SE 照片报「内容展示重叠，无法滑动」。

## 现象

- 三行身份选项里最后一行被贴底的 Continue 胶囊盖住一半；往上滑没有任何反应。
- 只在 SE 这类小屏出现；大屏（中间区更高）完全正常，所以模拟器默认机型走查不到。

## 根因

`OnboardingScaffold` 的中间区是 `min-h-0 flex-1` 的 **普通 `View`**：

- Yoga 的 `flex-1` 只是「分到剩余高度」，**不会裁剪**子元素，也不产生滚动（`View` 默认 `overflow: visible`）。
- 这一屏内容 = 仓鼠 `mt-6` + 112 + 三行固定 `64pt`（`gap-4`）+ `mt-[34px]` ≈ 394pt；375×667（SE）上中间区只有约 374pt → 溢出约 20pt，正好画到 CTA 底下，看起来像被按钮压住。
- 行高是**固定值**（`IdentityOptionRow` 的 `height: 64`），不会随剩余高度收缩，所以不是「挤一挤能放下」的类型。

## 修法

把内容区换成 `ScrollView`（大屏不滚、小屏可滚），CTA 保持钉在滚动区外：

```tsx
<ScrollView
  className="mt-[34px] min-h-0 flex-1"
  contentContainerClassName="gap-4 pb-4"   // 末行下方留出可滚出来的余量
  showsVerticalScrollIndicator={false}
>
  {/* 固定高度的行 */}
</ScrollView>
```

同仓库同构的先例：`onboarding/{looking-for,faith,drink,smoke,who-you-meet}.tsx` —— 它们的选项数量更多，本来就带着 `ScrollView`，只有 gender 这一屏漏了。

## 第二个实例：照片编辑页把照片放大之后（2026-09-22 当天）

`sitin-rn` `apps/luka/src/app/edit-profile/photos.tsx`（A 版 redesign）：照片位从
`200×240` 的固定小块换成**通栏竖幅**（`aspectRatio: 1/1.2`，宽 = 屏宽 − 40），页面总高
在 390×844 上刚好、在 375×667（SE）上超出 → 同样的静默溢出。

差别只在判据怎么用：这一屏不是「固定高度的列表」，而是**一个随屏宽等比放大、且本身
尺寸可观的元素**（照片 / 视频 / 大插画）。`aspectRatio` 让它宽高一起长，窄屏上并不会
变小多少，所以「大屏没事」这句安慰话在这里是**假**的 —— 大屏只是刚好够。

修法同样是内容区 `ScrollView`，但多一条：**贴底的 Save 必须留在 `ScrollView` 之外**，
滚的只有内容（照片再高也不能把按钮顶走）。结构由
`tests/design/edit-profile-photos.test.ts` 守着：

```tsx
<ScrollView className="min-h-0 flex-1" contentContainerClassName="pb-4">
  {/* eyebrow / 标题 / 照片 / AI tip */}
</ScrollView>
<View className="px-5 pt-4" style={{ paddingBottom: … }}>
  <ContinueButton … />
</View>
```

## 判据 / 检查法

- 屏结构是「可伸缩内容区 + 贴底固定栏（CTA / tab bar）」时，问一句：**内容会不会在任何机型上超过内容区？** 会，就必须 `ScrollView`；固定高度的列表项（卡片、行、ruler）尤其危险。
- 自查尺寸用真实最小机型（SE 375×667 / 320×568）算一遍：`安全区 + header + eyebrow + 标题 + 副标题 + 装饰 + 列表` 是否超过可用高度。别用默认模拟器机型拍板。
- 这类溢出**不会报错、不会裁切**，只会静默叠在贴底栏下面，所以只能靠机型走查或尺寸推算发现。

## 相关

- 键盘场景的同类问题（输入框被钉住的操作栏盖住）：[[rn-keyboard-and-pinned-footer]]。
- luka 的形状/布局守门测试：[[sitin-rn-luka-design-guard-tests]]。
