const url = window.__TAURI__.tauri.convertFileSrc(
  `file/BigBuckBunny.mp4`,
  "spacedrive"
);

console.log(url);

const iframe = document.createElement("iframe");
iframe.src = url;
iframe.style.width = "100%";
iframe.style.height = "100%";
document.body.appendChild(iframe);
