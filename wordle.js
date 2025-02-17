// Share text to the clipboard
export function share(text) {
    if (navigator.share) {
        navigator.share({ text })
            .then(() => console.log("Successful share"))
            .catch((error) => console.log("Error sharing", error));
    } else if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text);
    } else {
        document.getElementById("shared").innerText = text;
    }
}
