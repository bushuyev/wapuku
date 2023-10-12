/*! coi-serviceworker v0.1.7 - Guido Zuidhof and contributors, licensed under MIT */
let coepCredentialless = false;
if (typeof window === 'undefined') {
    self.addEventListener("install", () =>  {
        console.log("service worker install");
    });
    self.addEventListener("activate", (event) =>  {
        console.log("service worker activate");
    });

    self.addEventListener("message", (ev) => {
        console.log("service worker message");
    });

    self.addEventListener("fetch", function (event) {
        console.log("service worker fetch: event"+event);
        event.respondWith(
            fetch(event.request)
                .then((response) => {
                    if (response.status === 0) {
                        return response;
                    }

                    const newHeaders = new Headers(response.headers);
                    newHeaders.set("Access-Control-Allow-Origin", "https://bushuyev.github.io");
                    newHeaders.set("Cross-Origin-Embedder-Policy", "credentialless");
                    newHeaders.set("Cross-Origin-Resource-Policy", "same-site");
                    newHeaders.set("Cross-Origin-Opener-Policy", "same-origin");

                    return new Response(response.body, {
                        status: response.status,
                        statusText: response.statusText,
                        headers: newHeaders,
                    });
                })
                .catch((e) => console.error(e))
        )
    });

} else {
    (() => {

        navigator.serviceWorker.getRegistration().then(function(registration) {
            if (!registration || !navigator.serviceWorker.controller) {
                navigator.serviceWorker.register('./service-worker.js').then(function() {
                    console.log('Service worker registered, reloading the page');
                    window.location.reload();
                });
            } else {
                console.log('DEBUG: client is under the control of service worker');

            }
        });
    })();
}
