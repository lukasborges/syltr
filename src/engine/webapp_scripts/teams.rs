//! Microsoft Teams-specific UI fixes.
//!
//! Suppresses the "Something went wrong / can't receive or make calls" modal
//! that appears because WebKit has WebRTC disabled.

pub const TEAMS_JS: &str = r#"
(function () {
  const MODAL_TITLE = "Something went wrong";
  const MODAL_BODY  = "can't receive or make calls";

  const suppressModal = function (node) {
    if (!node || node.nodeType !== Node.ELEMENT_NODE) return;
    const text = node.textContent || "";
    if (!text.includes(MODAL_TITLE) || !text.includes(MODAL_BODY)) return;

    // Walk up to the highest dialog / fixed container before body.
    let target = node;
    let walk = node;
    while (walk && walk !== document.body) {
      const role = walk.getAttribute("role");
      const pos = window.getComputedStyle(walk).position;
      if (role === "dialog" || role === "alertdialog" || pos === "fixed") {
        target = walk;
      }
      walk = walk.parentElement;
    }
    if (target && target !== document.body) {
      console.log("[syltr/teams] suppressing call error modal");
      target.remove();

      // Also remove any backdrop overlay sibling.
      const parent = target.parentElement;
      if (parent) {
        Array.from(parent.children).forEach(function (sib) {
          const sibStyle = window.getComputedStyle(sib);
          if (sib !== target && sibStyle.position === "fixed" && sibStyle.pointerEvents !== "none") {
            sib.remove();
          }
        });
      }
    }
  };

  const observer = new MutationObserver(function (mutations) {
    for (let i = 0; i < mutations.length; i++) {
      const added = mutations[i].addedNodes;
      for (let j = 0; j < added.length; j++) {
        suppressModal(added[j]);
      }
    }
  });

  const start = function () {
    if (!document.body) return;
    // Remove any modal already in the DOM.
    const dialogs = document.querySelectorAll("[role='dialog'], [role='alertdialog']");
    for (let i = 0; i < dialogs.length; i++) {
      suppressModal(dialogs[i]);
    }
    observer.observe(document.body, { childList: true, subtree: true });
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
"#;
