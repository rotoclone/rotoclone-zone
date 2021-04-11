// from https://css-tricks.com/a-complete-guide-to-dark-mode-on-the-web/

const btn = document.querySelector(".theme-toggle");

btn.addEventListener("click", function () {
    document.body.classList.toggle("light-theme");
    var theme = document.body.classList.contains("light-theme") ? "light" : "dark";
    localStorage.setItem("theme", theme);
});