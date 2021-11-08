import {AfterViewInit, Component, OnInit} from '@angular/core';
// @ts-ignore
import * as L from 'leaflet';
import { GraphServiceService } from '../service/graph-service.service';
import { SimulationConfig } from '../data/SimulationConfig';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { MatDialog } from '@angular/material/dialog';
import { FormControl, Validators } from '@angular/forms';

@Component({
  selector: 'app-graph-viewer',
  templateUrl: './graph-viewer.component.html',
  styleUrls: ['./graph-viewer.component.css']
})
export class GraphViewerComponent implements OnInit, AfterViewInit {
  private map: any;

  private simConfig: SimulationConfig;
  turnControl: FormControl;
  currentTurn = 0;
  maxTurn = 0;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
    this.turnControl = new FormControl(0, [Validators.required]);
  }

  ngOnInit(): void {
  }

  ngAfterViewInit(): void {
    this.initMap();
  }

  openSimulationConfigDialog() {

    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px'
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      this.simConfig = data;
      this.graphservice.simulate(this.simConfig).subscribe(response => {
        console.log(response)
        this.maxTurn = response.end_time;
      });
    })
  }

  private initMap(): void {
    this.map = L.map('map', {
      center: [39.8282, -98.5795],
      zoom: 3
    });
    const tiles = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 18,
      minZoom: 3,
      attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
    });

    tiles.addTo(this.map);
  }
}
