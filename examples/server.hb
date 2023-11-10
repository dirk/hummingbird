// Let's write a reloading HTTP server in 20 lines of code!

// server.hb
import "fs/watch"
import "net/http/server"
import "vm"

func main() {
    let module = vm.resolve("router.hb")
    var router = vm.load(module)!
    watch.module(module, fn(changeset) => {
        router = vm.load(module, changeset: changeset)!
    })
    http.listenAndServe(":8080", fn(req, res) => {
        router.exports["route"](req, res)
    })
}

// router.hb
import { Request, Response } from "net/http/server"

export func route(req: Request, res: Response) {
    res.end(200, "Hello world!\n")
}
