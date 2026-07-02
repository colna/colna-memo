---
title: sitin-next app-pwa — PhotoTaskDrawer(探真拍照抽屉)使用方式
date: 2026-07-02
tags: [sitin-next, app-pwa, component, phototask, 探真]
---

# PhotoTaskDrawer 使用方式

「探真」拍照验证底部抽屉。文件 `packages/app-pwa/src/pages/ChatDetail/PhotoTaskDrawer.tsx`(PR #517,分支 `personal/zz/pwa-verify-drawer`)。

## Props

```ts
interface PhotoVerifyResult {
  passed: boolean;
  cdnUrl?: string;      // 通过时给出,用于后续发消息
  reasons?: string[];   // 失败原因,直接渲染成红 chip(如 ["No face detected","Not a female face"])
}

interface PhotoTaskDrawerProps {
  open: boolean;
  onClose: () => void;
  peerName: string;                 // 标题里的对方名,如 "Jake"
  rewardText?: string;              // 奖励 chip,默认 "Earn +¢1.20"
  countdownSeconds?: number;        // 倒计时秒数(idle 头部 mm:ss),默认 180
  onVerify: (file: File) => Promise<PhotoVerifyResult>;  // 注入:上传+审核
  onSent?: (cdnUrl: string) => void;                     // 审核通过并"发送"后回调
}
```

## 状态机(内部自管,无需外部驱动)

`idle(待拍照) → ready(待发送) → reviewing(审核中) → failed(不通过)`

- **idle**:虚线框 / "Open camera" → 调**前置系统相机**(`<input type="file" accept="image/*" capture="user">`,跳相册)。
- **ready**:展示刚拍的照片(按原始比例 `object-contain`,`max-h-420`,不裁切)+ 3 个绿色 pre-send chip + "Send photo"。**点照片本身 = 重开相机重拍/重新上传**。
- **reviewing**:点 Send 后进入,照片叠半透明遮罩 + Loading 胶囊,调用 `onVerify(file)`。
- **通过** → `onSent(cdnUrl)` + `onClose()`;**失败** → `failed`,红 chip 列 `reasons`,"Re-upload" 重开相机。

设计要点:ready/reviewing **共用同一个 `<img>` DOM 节点**(reviewing 只叠覆盖层),避免切态 remount 导致重新 decode 闪一下 —— 见 [[sitin-next-pwa-figma-webp]] 同批工作的踩坑记录。图标全走 webp。

## 接真实后端(ChatDetail 用)

`onVerify` 里串 OSS 直传 + 数美审核(注意:审核 proto `archat_api/chat_api` 后端**尚未落地**,须等 `chat_api.proto` + `pnpm proto:gen`):

```ts
import { uploadToOss } from "@/utils/ossUpload";
import { FileType } from "@/http/ossUploadApi";
import { auditImage, ViolationCategory } from "@/http/chatModerationApi";

const onVerify = async (file: File): Promise<PhotoVerifyResult> => {
  const cred = await uploadToOss({
    file, fileType: FileType.IMAGE, fileExt: "jpg",
    contentType: file.type || "image/jpeg",
  });
  const audit = await auditImage({ userId, imageUrl: cred.cdnUrl, targetId: peerUserId });
  if (audit.passed) return { passed: true, cdnUrl: cred.cdnUrl };
  const map: Record<number, string> = {
    [ViolationCategory.NO_HUMAN_FACE]: "No face detected",
    [ViolationCategory.NOT_FEMALE]: "Not a female face",
  };
  return { passed: false, reasons: [map[audit.violationCategory] ?? "Photo rejected"] };
};
```

触发点(下一轮):`MessageType.RequestSelfie` 消息(`MessageItem.tsx` 现在只渲染占位),或 `ChatDetail/index.tsx` 里加 `showPhotoTask` state;倒计时到期时间 / 奖励 / taskId 的后端数据源待定。

## Dev 预览

`/dev/photo-task`(`pages/PhotoTaskPreview/index.tsx`),mock `onVerify` 可切通过/失败,点通全流程,不依赖后端。
