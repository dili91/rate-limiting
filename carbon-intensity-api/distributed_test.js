import http from 'k6/http';
import { Counter } from 'k6/metrics';
import { sleep } from 'k6';

const api_responses = new Counter('api_responses');

export const options = {
    vus: 25,
    iterations: 100,
    thresholds: {
        'api_responses': [
            'count == 100'
        ],
        'api_responses{status:429}': [
            'count == 95'
        ],
        'api_responses{status:200}': [
            'count == 5'
        ],
    },
    'summaryTrendStats': ['min', 'med', 'avg', 'p(90)', 'p(95)', 'max', 'count'],
};

export default function () {
    const res = http.get('http://localhost:8080/carbon/intensity');

    api_responses.add(1, {status: res.status})

    sleep(1);
}