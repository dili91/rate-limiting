import http from 'k6/http';
import { Counter } from 'k6/metrics';
import { check } from 'k6';


const api_responses = new Counter('api_responses');

export const options = {
    vus: 25,
    iterations: 500,
    thresholds: {
        'api_responses': [
            'count == 500'
        ],
        'api_responses{status:429}': [
            'count == 495'
        ],
        'api_responses{status:200}': [
            'count == 5'
        ],
    },
};

export default function () {
    const res = http.get('http://localhost:8080/carbon/intensity');

    api_responses.add(1, {status: res.status})

    const output = check(res, {
        'Status code is either 200 or 429': (r) => r.status === 200 || r.status === 429,
    });
}