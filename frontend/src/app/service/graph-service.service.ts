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
    return this.http.post(this.path + "/simulate",config ,{withCredentials: true});
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
