import {createStore, applyMiddleware} from 'redux'
import {attach} from 'khufu-runtime'
import * as websock from './websock'
import {} from './graphql'

import {main} from './main.khufu'
import {router} from './util/routing'

let khufu_instance = attach(document.getElementById('app'), main(VERSION), {
    store(reducer, middleware, state) {
        let mid = middleware.filter(x => typeof x === 'function')
        if(DEBUG) {
            let logger = require('redux-logger')
            mid.push(logger.createLogger({
                collapsed: true,
            }))
        }
        let store = createStore(reducer, state, applyMiddleware(...mid))
        for(var m of middleware) {
            if(typeof m !== 'function') {
                if(m.type) {
                    store.dispatch(m)
                } else if(DEBUG) {
                    console.error("Wrong middleware", m)
                    throw Error("Wrong middleware: " + m)
                }
            }
        }
        return store
    }
})

let unsubscribe = router.subscribe(khufu_instance.queue_render)

if(!DEBUG) {
    websock.start('ws://' + location.host  + '/ws',
        khufu_instance.queue_render)
} else {
    if(location.port != 8080) {
        websock.start('ws://' + location.host + '/ws',
            khufu_instance.queue_render)
    } else {
        websock.start('ws://' + location.hostname  + ':22682/ws',
            khufu_instance.queue_render)
    }
}

if(module.hot) {
    module.hot.accept()
    module.hot.dispose(() => {
        unsubscribe()
        websock.stop()
    })
}
