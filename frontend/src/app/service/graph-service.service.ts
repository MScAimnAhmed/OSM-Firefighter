import { Injectable } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import {Observable} from "rxjs";
import { SimulationConfig } from '../data/SimulationConfig';

@Injectable({
  providedIn: 'root'
})
export class GraphServiceService {
  private path = "http://localhost:8080";

  constructor(protected http: HttpClient) { }


  ping(): Observable<any> {
    return this.http.get("http://localhost:8080/ping");
  }

  getGraphs(): Observable<any> {
    return this.http.get(this.path + "/graphs");
  }

  simulate(config: SimulationConfig): Observable<any> {
    let params = new HttpParams()
      .append('graph', config.graph)
      .append('strategy', config.strategy)
      .append('num_ffs', String(config.num_ffs))
      .append('num_roots', String(config.num_roots));
    return this.http.post(this.path + "/simulate",null ,{ params: params });
  }
}
