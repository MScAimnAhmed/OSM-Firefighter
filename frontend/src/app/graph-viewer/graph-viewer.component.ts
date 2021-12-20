import { Component, OnInit, ViewChild } from '@angular/core';
import { GraphServiceService } from '../service/graph-service.service';
import { SimulationConfig } from '../data/SimulationConfig';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { MatDialog } from '@angular/material/dialog';
import { TurnInputComponent } from '../view-inputs/turn-input/turn-input.component';
import { ViewInputComponent } from '../view-inputs/view-input/view-input.component';
import { ZoomInputComponent } from '../view-inputs/zoom-input/zoom-input.component';
import { MetaInfoBoxComponent } from '../meta-info-box/meta-info-box.component';
import { SimulationMetaData } from '../data/SimulationMetaData';

@Component({
  selector: 'app-graph-viewer',
  templateUrl: './graph-viewer.component.html',
  styleUrls: ['./graph-viewer.component.css']
})
export class GraphViewerComponent implements OnInit {

  @ViewChild(TurnInputComponent) turnInput: TurnInputComponent;
  @ViewChild(ViewInputComponent) viewInput: ViewInputComponent;
  @ViewChild(ZoomInputComponent) zoomInput: ZoomInputComponent;
  @ViewChild(MetaInfoBoxComponent) infoBox: MetaInfoBoxComponent;

  refreshing: boolean;
  pending: boolean;
  activeSimulation: boolean;
  simConfig: SimulationConfig;

  thumbnail: any;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
  }

  ngOnInit(): void {
  }

  openSimulationConfigDialog() {
    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px',
      data: this.simConfig
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      if (data) {
        this.simConfig = data;
        this.graphservice.simulate(data).subscribe((response: SimulationMetaData) => {
          this.activeSimulation = true;
          this.turnInput.currentTurn = 0;
          this.turnInput.maxTurn = response.end_time;
          this.viewInput.currentCoord.lat = response.view_center[0];
          this.viewInput.currentCoord.lon = response.view_center[1];
          this.viewInput.maxCoord.lat = response.view_bounds.max_lat;
          this.viewInput.minCoord.lat = response.view_bounds.min_lat;
          this.viewInput.maxCoord.lon = response.view_bounds.max_lon;
          this.viewInput.minCoord.lon = response.view_bounds.min_lon;
          this.zoomInput.currentZoom = 1;
          this.refreshView();
        });
      }
    });
  }

  public refreshView() {
    if (this.activeSimulation) {
      this.infoBox.updateStepMetaData(this.turnInput.currentTurn);
      if (!this.refreshing) {
        this.refreshing = true;this.graphservice.refreshView(this.turnInput.currentTurn, this.zoomInput.currentZoom, this.viewInput.currentCoord)
          .subscribe((data: Blob) => {
            this.refreshing = false;
            this.createImageFromBlob(data);
            if (this.pending) {
              this.pending = false;
              this.refreshView();
            }
          }, _ => {
            console.log('Could not refresh the View');
            this.refreshing = false;
          });
      } else {
        this.pending = true;
      }
    }
  }

  createImageFromBlob(image: Blob) {
    let reader = new FileReader();
    reader.addEventListener('load', () => {
      this.thumbnail = reader.result;
    }, false);

    if (image) {
      reader.readAsDataURL(image);
    }
  }
}
