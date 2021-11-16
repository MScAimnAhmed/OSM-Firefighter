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

  getStrategies(): Observable<any> {
    return this.http.get(this.path + "/strategies")
  }

  simulate(config: SimulationConfig): Observable<any> {
    let params = new HttpParams()
      .append('graph', config.graph)
      .append('strategy', config.strategy)
      .append('num_ffs', String(config.num_ffs))
      .append('num_roots', String(config.num_roots))
      .append('strategy_every', String(config.strategy_every));
    return this.http.post(this.path + "/simulate",null ,{ params: params, withCredentials: true});
  }

  refreshView(turnNumber?: number, zoomLevel? : number) : Observable<Blob>{
    let params = new HttpParams();
    if(turnNumber) {
      params.append('time', turnNumber);
    }
    if(zoomLevel) {
      params.append('zoom', zoomLevel)
    }
    return this.http.get(this.path + "/view", {params: params, withCredentials: true, responseType: 'blob'});
  }
}
