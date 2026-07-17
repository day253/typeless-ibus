const translations = {
  zh: {
    navExperience: "体验",
    navFeatures: "特点",
    navSupport: "支持",
    heroEyebrow: "Linux 原生 · IBus · Wayland",
    heroTitle: "说出来，<br><em>文字就到了。</em>",
    heroLede: "按住一个键，开口说，松开。语音直接变成当前应用里的文字，像输入法本来就该有的能力。",
    download: "下载 v0.5.2",
    seeDemo: "看看怎么用",
    noLlm: "无 LLM",
    noClipboard: "无剪贴板注入",
    desktopActivities: "活动",
    notesTitle: "今天的笔记",
    noteDate: "星期六 · 7月18日",
    noteHeading: "周末计划",
    listening: "聆听中…",
    status: "状态",
    storyEyebrow: "一次顺手的输入",
    storyTitle: "不用离开键盘。<br>也不用打断思路。",
    storyIntro: "向下滑，看看一句话如何自然地进入 Ubuntu 当前窗口。",
    chatTitle: "团队聊天",
    messageOne: "下午要不要一起去公园？",
    messageTwo: "好呀，我安排一下时间。",
    ready: "准备好了",
    hold: "按住",
    holdHint: "也可以按住这里试一试",
    stepOneTitle: "光标放在任何输入框",
    stepOneBody: "聊天、浏览器、笔记或终端。Typeless 是输入法，不挑应用。",
    stepTwoTitle: "按住 Fn，开始说",
    stepTwoBody: "轻巧的“聆听中…”告诉你它已经准备好，不用寻找录音窗口。",
    stepThreeTitle: "文字跟着声音生长",
    stepThreeBody: "识别结果以原生 IBus 预编辑文本出现，长句和停顿也不会丢掉前文。",
    stepFourTitle: "松开，直接写入",
    stepFourBody: "最终文字提交到当前光标，不经过剪贴板，也不模拟粘贴。",
    featuresEyebrow: "小而专注",
    featuresTitle: "只做好语音输入。",
    featuresIntro: "没有账号、历史记录或 LLM 改写。一个安静的输入法，留在系统该在的位置。",
    nativeTitle: "原生 IBus",
    nativeBody: "通过发行版已有的 GTK、Qt、XIM 与 Wayland 模块覆盖日常应用。",
    rustTitle: "轻量 Rust 核心",
    rustBody: "不依赖 Python、GTK 设置程序或常驻网页服务，安装后安静运行。",
    directTitle: "直接提交文字",
    directBody: "不读取和覆盖剪贴板，不注入快捷键；音频仅用于实时语音识别。",
    supportEyebrow: "从旧机器到新机器",
    supportTitle: "你的 Linux，<br>大概率已经准备好了。",
    supportBody: "支持 IBus 1.5.22 及以上版本，提供 x86_64 与 ARM64 原生安装包。",
    choosePackage: "选择安装包",
    tested: "协议测试",
    finalEyebrow: "开口就好",
    finalTitle: "下一句话，不用打字。",
    finalBody: "安装 typeless-ibus，把声音放进正在使用的输入框。",
    downloadLinux: "下载 Linux 安装包",
    readSource: "查看源代码",
    footer: "为 Linux 做的一件小事 · Rust + IBus · MIT License",
    demoReady: "准备好了",
    demoListening: "聆听中…",
    demoRecognizing: "正在识别",
    demoInserted: "已写入",
    demoPartial: "今天下午三点，",
    demoFinal: "今天下午三点，我们一起去公园散步吧。",
  },
  en: {
    navExperience: "Experience",
    navFeatures: "Features",
    navSupport: "Support",
    heroEyebrow: "Native Linux · IBus · Wayland",
    heroTitle: "Say it.<br><em>There’s your text.</em>",
    heroLede: "Hold a key, speak, and release. Your voice becomes text in the app you are already using—just like an input method should.",
    download: "Download v0.5.2",
    seeDemo: "See how it feels",
    noLlm: "No LLM",
    noClipboard: "No clipboard injection",
    desktopActivities: "Activities",
    notesTitle: "Today’s notes",
    noteDate: "Saturday · July 18",
    noteHeading: "Weekend plan",
    listening: "Listening…",
    status: "Status",
    storyEyebrow: "Input that stays out of the way",
    storyTitle: "Stay on the keyboard.<br>Stay in your thought.",
    storyIntro: "Scroll to follow one sentence into the active Ubuntu window.",
    chatTitle: "Team chat",
    messageOne: "Want to go to the park this afternoon?",
    messageTwo: "Sounds good. I’ll check my schedule.",
    ready: "Ready",
    hold: "Hold",
    holdHint: "Or press and hold here to try it",
    stepOneTitle: "Focus any text field",
    stepOneBody: "Chat, browser, notes, or terminal. Typeless is an input method, so it works across apps.",
    stepTwoTitle: "Hold Fn and start talking",
    stepTwoBody: "A quiet “Listening…” tells you it is ready. There is no recorder window to find.",
    stepThreeTitle: "Text grows with your voice",
    stepThreeBody: "Results appear as native IBus preedit text. Long sentences and pauses keep their earlier words.",
    stepFourTitle: "Release to write",
    stepFourBody: "Final text lands at the cursor—without touching the clipboard or simulating paste.",
    featuresEyebrow: "Small and focused",
    featuresTitle: "Voice input. Done well.",
    featuresIntro: "No accounts, history, or LLM rewriting. Just a quiet input method, living where it belongs.",
    nativeTitle: "Native IBus",
    nativeBody: "Works through the GTK, Qt, XIM, and Wayland modules already supplied by your distribution.",
    rustTitle: "Lightweight Rust core",
    rustBody: "No Python runtime, GTK settings app, or resident web service. It stays quiet after installation.",
    directTitle: "Direct text commit",
    directBody: "Never reads or replaces your clipboard and never injects shortcuts. Audio is used only for live ASR.",
    supportEyebrow: "Old machines and new ones",
    supportTitle: "Your Linux is<br>probably ready.",
    supportBody: "Supports IBus 1.5.22 and newer, with native packages for x86_64 and ARM64.",
    choosePackage: "Choose a package",
    tested: "protocol tested",
    finalEyebrow: "Just start talking",
    finalTitle: "Don’t type the next sentence.",
    finalBody: "Install typeless-ibus and put your voice into the field you are already using.",
    downloadLinux: "Download for Linux",
    readSource: "Read the source",
    footer: "One small thing for Linux · Rust + IBus · MIT License",
    demoReady: "Ready",
    demoListening: "Listening…",
    demoRecognizing: "Recognizing",
    demoInserted: "Inserted",
    demoPartial: "At three this afternoon, ",
    demoFinal: "At three this afternoon, let’s take a walk in the park.",
  },
};

const htmlTranslationKeys = new Set(["heroTitle", "storyTitle", "supportTitle"]);
const languageButton = document.querySelector(".language-toggle");
const languageCurrent = document.querySelector(".language-current");
const storyDesktop = document.querySelector(".story-desktop");
const storySteps = [...document.querySelectorAll(".story-step")];
const stageDots = [...document.querySelectorAll("[data-stage-target]")];
const composeText = document.querySelector(".compose-text");
const composePreedit = document.querySelector(".compose-preedit");
const composeCaret = document.querySelector(".compose-caret");
const stageStatus = document.querySelector(".stage-status-label");
const fnKey = document.querySelector(".fn-key");
let currentStage = 0;
let holdTimers = [];
let isHolding = false;

const preferredLanguage = () => {
  const saved = localStorage.getItem("typeless-language");
  if (saved === "zh" || saved === "en") return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
};

let currentLanguage = preferredLanguage();

function applyLanguage(language) {
  currentLanguage = language;
  const copy = translations[language];
  document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  document.title = language === "zh"
    ? "typeless-ibus — Linux 原生语音输入"
    : "typeless-ibus — Native voice input for Linux";

  document.querySelectorAll("[data-i18n]").forEach((element) => {
    const key = element.dataset.i18n;
    if (!(key in copy)) return;
    if (htmlTranslationKeys.has(key)) element.innerHTML = copy[key];
    else element.textContent = copy[key];
  });

  languageCurrent.textContent = language === "zh" ? "中" : "EN";
  languageButton.setAttribute("aria-label", language === "zh" ? "Switch to English" : "切换到中文");
  localStorage.setItem("typeless-language", language);
  renderStage(currentStage);
}

languageButton.addEventListener("click", () => applyLanguage(currentLanguage === "zh" ? "en" : "zh"));

function clearHoldTimers() {
  holdTimers.forEach(window.clearTimeout);
  holdTimers = [];
}

function renderStage(stage) {
  currentStage = Number(stage);
  const copy = translations[currentLanguage];
  storyDesktop.dataset.stage = String(currentStage);
  storySteps.forEach((step) => step.classList.toggle("active", Number(step.dataset.stage) === currentStage));
  stageDots.forEach((dot) => dot.classList.toggle("active", Number(dot.dataset.stageTarget) === currentStage));

  composeText.textContent = currentStage === 3 ? copy.demoFinal : "";
  composePreedit.textContent = currentStage === 1
    ? copy.demoListening
    : currentStage === 2
      ? copy.demoPartial
      : "";
  composeCaret.hidden = currentStage === 3;
  stageStatus.textContent = [copy.demoReady, copy.demoListening, copy.demoRecognizing, copy.demoInserted][currentStage];
}

const stepObserver = new IntersectionObserver(
  (entries) => {
    const visible = entries
      .filter((entry) => entry.isIntersecting)
      .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
    if (visible && !isHolding) renderStage(visible.target.dataset.stage);
  },
  { rootMargin: "-28% 0px -44%", threshold: [0.15, 0.4, 0.7] },
);

storySteps.forEach((step) => stepObserver.observe(step));

stageDots.forEach((dot) => {
  dot.addEventListener("click", () => {
    const target = storySteps[Number(dot.dataset.stageTarget)];
    target.scrollIntoView({ behavior: "smooth", block: "center" });
  });
});

function beginHold(event) {
  if (event) event.preventDefault();
  clearHoldTimers();
  isHolding = true;
  fnKey.classList.add("pressed");
  if (event?.pointerId !== undefined) fnKey.setPointerCapture?.(event.pointerId);
  renderStage(1);
  holdTimers.push(window.setTimeout(() => isHolding && renderStage(2), 650));
}

function endHold(event) {
  if (!isHolding) return;
  if (event) event.preventDefault();
  clearHoldTimers();
  isHolding = false;
  fnKey.classList.remove("pressed");
  renderStage(3);
}

fnKey.addEventListener("pointerdown", beginHold);
fnKey.addEventListener("pointerup", endHold);
fnKey.addEventListener("pointercancel", endHold);
fnKey.addEventListener("lostpointercapture", endHold);
fnKey.addEventListener("keydown", (event) => {
  if ((event.key === " " || event.key === "Enter") && !event.repeat && !isHolding) beginHold(event);
});
fnKey.addEventListener("keyup", (event) => {
  if (event.key === " " || event.key === "Enter") endHold(event);
});

const revealObserver = new IntersectionObserver(
  (entries) => entries.forEach((entry) => entry.isIntersecting && entry.target.classList.add("visible")),
  { rootMargin: "0px 0px -8%", threshold: 0.12 },
);
document.querySelectorAll(".reveal").forEach((element) => revealObserver.observe(element));

const header = document.querySelector(".site-header");
const updateHeader = () => header.classList.toggle("scrolled", window.scrollY > 12);
window.addEventListener("scroll", updateHeader, { passive: true });
updateHeader();

applyLanguage(currentLanguage);
renderStage(0);
