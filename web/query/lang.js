class Query {
    constructor(value) {
        Object.assign(this, value)
    }
    // Generic filter
    filter(cond) {
        return new Query({
            ...this,
            series: {
                ...this.series,
                condition: this.condition
                    ? ['And', this.condition, cond]
                    : cond,
            },
        })
    }
    // Filters
    matching(item, regex_str) {
        return this.filter(["RegexLike", item, regex_str])
    }
    non_matching(item, regex_str) {
        return this.filter(['Not', ['RegexLike', regex_str]])
    }

    // Extractors
    tip() {
        return new Query({
            ...this,
            extract: ['Tip'],
        })
    }
    history(num=1100) {
        return new Query({
            ...this,
            extract: ['HistoryByNum', 1100],
        })
    }
    // Generic function
    func(...item) {
        return new Query({
            ...this,
            functions: this.functions.concat(item),
        })
    }
    // Functions
    derivative() {
        return this.func('NonNegativeDerivative')
    }
    sumby(item, calc_total=true) {
        return this.func('SumBy', item, 'Ignore', calc_total)
    }
}

export function fine_grained() {
    return new Query({
        series: {source: 'Fine', condition: []},
        extract: null,
        functions: [],
    })

}