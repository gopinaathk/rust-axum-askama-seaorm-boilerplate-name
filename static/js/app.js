/**
 * Alpine.js components.
 *
 * This file is loaded before the Alpine bundle, so everything registers on the
 * `alpine:init` event. No build step, no bundler.
 */
document.addEventListener("alpine:init", () => {
  /** Dark/light switch, persisted in localStorage. */
  Alpine.data("themeToggle", () => ({
    isDark: document.documentElement.dataset.theme !== "light",

    get label() {
      return this.isDark ? "Switch to light theme" : "Switch to dark theme";
    },

    toggle() {
      this.isDark = !this.isDark;
      const theme = this.isDark ? "dark" : "light";
      document.documentElement.dataset.theme = theme;

      try {
        localStorage.setItem("theme", theme);
      } catch (error) {
        /* storage blocked: the theme still applies for this page */
      }
    },
  }));

  /** Collapsible navigation for narrow viewports. */
  Alpine.data("mobileNav", () => ({
    open: false,

    toggle() {
      this.open = !this.open;
    },
  }));

  /** Flash messages: closable, and success notices fade out on their own. */
  Alpine.data("dismissible", () => ({
    open: true,

    init() {
      if (this.$el.classList.contains("alert--success")) {
        this.timer = setTimeout(() => this.close(), 6000);
      }
    },

    destroy() {
      clearTimeout(this.timer);
    },

    close() {
      this.open = false;
    },
  }));

  /** Prevents double submits and shows progress copy on the button. */
  Alpine.data("submitGuard", () => ({
    busy: false,

    lock() {
      this.busy = true;
    },
  }));

  /** Show/hide toggle for a single password input. */
  Alpine.data("passwordField", () => ({
    visible: false,

    toggle() {
      this.visible = !this.visible;
    },
  }));

  /**
   * Password visibility plus a strength meter.
   * The meter is advisory only: the server enforces the real rules.
   */
  Alpine.data("passwordStrength", () => ({
    visible: false,
    value: "",

    toggle() {
      this.visible = !this.visible;
    },

    get score() {
      const value = this.value;
      if (!value) return 0;

      let score = 0;
      if (value.length >= 8) score += 1;
      if (value.length >= 12) score += 1;
      if (/[a-zA-Z]/.test(value) && /\d/.test(value)) score += 1;
      if (/[^a-zA-Z0-9]/.test(value)) score += 1;

      return score;
    },

    get percent() {
      return (this.score / 4) * 100;
    },

    get barClass() {
      if (this.score >= 4) return "meter__bar--strong";
      if (this.score >= 2) return "meter__bar--fair";
      return "";
    },

    get hint() {
      if (!this.value) return "Use at least 8 characters with a letter and a number.";
      if (this.score <= 1) return "Too weak: add length, letters and numbers.";
      if (this.score === 2) return "Acceptable. Longer is better.";
      if (this.score === 3) return "Good password.";
      return "Strong password.";
    },
  }));

  /** Copy-to-clipboard with a short confirmation state. */
  Alpine.data("copyable", () => ({
    copied: false,

    async copy(text) {
      try {
        await navigator.clipboard.writeText(text);
      } catch (error) {
        // Fallback for browsers without the async clipboard API.
        const area = document.createElement("textarea");
        area.value = text;
        area.setAttribute("readonly", "");
        area.style.position = "fixed";
        area.style.opacity = "0";
        document.body.appendChild(area);
        area.select();
        document.execCommand("copy");
        document.body.removeChild(area);
      }

      this.copied = true;
      clearTimeout(this.copiedTimer);
      this.copiedTimer = setTimeout(() => {
        this.copied = false;
      }, 1800);
    },
  }));
});
