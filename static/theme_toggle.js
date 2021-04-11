// from https://css-tricks.com/a-complete-guide-to-dark-mode-on-the-web/

// Select the button
const btn = document.querySelector(".theme-toggle");

// Listen for a click on the button
btn.addEventListener("click", function () {
    document.body.classList.toggle("light-theme");
    var theme = document.body.classList.contains("light-theme") ? "light" : "dark";
    // Finally, let's save the current preference to localStorage to keep using it
    localStorage.setItem("theme", theme);
});