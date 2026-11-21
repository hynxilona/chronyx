// Small, dependency-free behaviour for the Chronyx site.
// Nothing here talks to the network: the site stays static on purpose.

(function () {
  "use strict";

  var root = document.documentElement;

  /* theme ------------------------------------------------------------- */

  var stored = null;
  try { stored = localStorage.getItem("chronyx-theme"); } catch (e) {}
  if (stored === "light" || stored === "dark") root.setAttribute("data-theme", stored);

  var toggle = document.querySelector("[data-theme-toggle]");
  if (toggle) {
    var label = function () {
      var light = root.getAttribute("data-theme") === "light";
      toggle.textContent = light ? "Dark" : "Light";
      toggle.setAttribute("aria-label", light ? "Switch to dark theme" : "Switch to light theme");
    };
    label();
    toggle.addEventListener("click", function () {
      var next = root.getAttribute("data-theme") === "light" ? "dark" : "light";
      root.setAttribute("data-theme", next);
      try { localStorage.setItem("chronyx-theme", next); } catch (e) {}
      label();
    });
  }

  /* nav highlight ----------------------------------------------------- */

  var page = document.body.getAttribute("data-page");
  if (page) {
    var current = document.querySelector('.nav a[data-nav="' + page + '"]');
    if (current) current.setAttribute("aria-current", "page");
  }

  /* small-screen menu -------------------------------------------------- */

  var menu = document.querySelector(".menu");
  if (menu) {
    menu.addEventListener("click", function (e) {
      if (e.target.closest("a")) menu.removeAttribute("open");
    });
    document.addEventListener("click", function (e) {
      if (menu.hasAttribute("open") && !menu.contains(e.target)) {
        menu.removeAttribute("open");
      }
    });
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && menu.hasAttribute("open")) {
        menu.removeAttribute("open");
        var summary = menu.querySelector("summary");
        if (summary) summary.focus();
      }
    });
  }

  /* copy buttons ------------------------------------------------------ */

  document.querySelectorAll("[data-copy]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var id = btn.getAttribute("data-copy");
      var src = document.getElementById(id);
      if (!src) return;
      var text = src.innerText.trim();
      var done = function () {
        var old = btn.textContent;
        btn.textContent = "copied";
        setTimeout(function () { btn.textContent = old; }, 1400);
      };
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(done, function () {});
      } else {
        var area = document.createElement("textarea");
        area.value = text;
        document.body.appendChild(area);
        area.select();
        try { document.execCommand("copy"); done(); } catch (e) {}
        document.body.removeChild(area);
      }
    });
  });

  /* PoH demo ---------------------------------------------------------- */

  var run = document.querySelector("[data-poh-run]");
  if (run) {
    var list = document.getElementById("poh-ticks");
    var stat = document.getElementById("poh-stat");
    var ticks = 8;

    var hex = function (buffer) {
      return Array.prototype.map.call(new Uint8Array(buffer), function (b) {
        return b.toString(16).padStart(2, "0");
      }).join("");
    };

    var tick = async function (seed) {
      var bytes = new TextEncoder().encode(seed);
      var digest = await crypto.subtle.digest("SHA-256", bytes);
      return hex(digest);
    };

    run.addEventListener("click", async function () {
      if (!window.crypto || !crypto.subtle) {
        stat.textContent = "WebCrypto is unavailable here, so this demo cannot run.";
        return;
      }
      run.disabled = true;
      list.innerHTML = "";
      var seed = "chronyx:" + Date.now();
      var started = performance.now();
      var hash = seed;
      for (var i = 1; i <= ticks; i++) {
        hash = await tick(hash);
        var li = document.createElement("li");
        var n = document.createElement("b");
        n.textContent = String(i).padStart(2, "0");
        var h = document.createElement("span");
        h.className = "h";
        h.textContent = hash.slice(0, 40);
        li.appendChild(n);
        li.appendChild(h);
        list.appendChild(li);
      }
      var elapsed = performance.now() - started;
      stat.textContent =
        ticks + " ticks = " + ticks * 50 + " ms of claimed history, chained in " +
        elapsed.toFixed(2) + " ms. Any node can replay the same chain and get the same order.";
      run.disabled = false;
    });
  }

  /* supply calculator ------------------------------------------------- */

  var input = document.getElementById("calc-in");
  var outQuanta = document.getElementById("calc-quanta");
  var outChronyx = document.getElementById("calc-chronyx");
  var QUANTA = 1000000000;

  if (input && outQuanta && outChronyx) {
    var render = function () {
      var raw = input.value.trim().replace(/,/g, "").replace(/_/g, "");
      var value = Number(raw);
      if (!raw || !isFinite(value) || value < 0) {
        outQuanta.textContent = "—";
        outChronyx.textContent = "—";
        return;
      }
      var quanta = Math.round(value * QUANTA);
      outQuanta.textContent = quanta.toLocaleString("en-US");
      outChronyx.textContent = (quanta / QUANTA).toLocaleString("en-US", { maximumFractionDigits: 9 });
    };
    input.addEventListener("input", render);
    render();
  }
})();
