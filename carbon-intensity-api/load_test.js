import http from 'k6/http';
import { sleep } from 'k6';

export const options = {
    vus: 100,
    iterations: 1000,
    thresholds: {
        // Some dummy thresholds that are always going to pass.
        'http_req_duration{status:200}': ['max>=0'],
        'http_req_duration{status:429}': ['max>=0'],
    },
    'summaryTrendStats': ['min', 'med', 'avg', 'p(90)', 'p(95)', 'max', 'count'],
};

export default function () {
    http.get('http://localhost:8080/intensity');
    sleep(1);
}