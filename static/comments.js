const showCommentsButton = document.querySelector(".show-comments-button");

window.showComments = function () {
    window.commento.main();
    showCommentsButton.style.display = "none";
}

window.loadedCommento = function () {
    if (window.location.href.includes("#commento")) {
        window.showComments();
        // TODO delay doing this until the element exists (but don't do it if it's too long after the page was loaded)
        document.querySelector("#" + window.location.href.split("#")[1]).scrollIntoView();
    } else {
        showCommentsButton.addEventListener("click", function () {
            window.showComments();
        });
    }
}

window.showCommentsButtonText = function (count) {
    if (count === 0) {
        return "Make a comment";
    } else if (count === 1) {
        return "Show 1 comment";
    } else {
        return "Show " + count + " comments";
    }
}