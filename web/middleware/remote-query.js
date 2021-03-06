import {applyMiddleware, createStore} from 'redux'
import {refresher, probor} from '../middleware/request'
import {UPDATE_REQUEST, DATA, ERROR} from '../middleware/request'
import {CANCEL} from 'khufu-runtime'
import {hosts_response} from '../stores/query'
import {decode} from '../util/probor'

export const METRICS = '@@remote-query/metrics'

var counter = 0
var queries = {}
var stores = new Map()


let update = refresher({})(action => {
    switch(action.type) {
        case DATA: {
            for(let [key, store] of stores.entries()) {
                let val = new Map()
                for(let [host, metrics] of action.data.entries()) {
                    val.set(host, metrics.get(key))
                }
                store.dispatch({type: METRICS, metrics: val})
            }
            break;
        }
        case ERROR: {
            for(let store of stores.values()) {
                /// TODO(tailhook) capture errors? middleware?
                store.dispatch(action)
            }
        }
    }
})

function add_query(id, query, store) {
    queries[id] = query
    stores.set(id, store)
    update(probor('/remote/query_by_host.cbor', hosts_response, 6000, {
        body: JSON.stringify({"rules": queries}),
        immediate: true,
    }))
}

function del_query(id) {
    delete queries[id]
    stores.delete(id)
    if(stores.size > 0) {
        update(probor('/remote/query_by_host.cbor', hosts_response, 6000, {
            body: JSON.stringify({"rules": queries}),
        }))
    } else {
        update({type: CANCEL})
    }
}

export var query = query => store => next => {
    let id = 'q' + (++counter);
    add_query(id, query, store)
    return action => {
        if(action.type == CANCEL) {
            del_query(id)
        }
        next(action)
    }
}



