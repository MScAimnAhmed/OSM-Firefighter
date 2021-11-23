import { AfterViewInit, Component, OnInit, ViewChild } from '@angular/core';
import { GraphServiceService } from '../service/graph-service.service';
import { SimulationConfig } from '../data/SimulationConfig';
import { SimulationConfiguratorComponent } from '../simulation-configurator/simulation-configurator.component';
import { MatDialog } from '@angular/material/dialog';
import { FormControl, Validators } from '@angular/forms';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';
import { TurnInputComponent } from '../view-inputs/turn-input/turn-input.component';
import { ViewInputComponent } from '../view-inputs/view-input/view-input.component';

@Component({
  selector: 'app-graph-viewer',
  templateUrl: './graph-viewer.component.html',
  styleUrls: ['./graph-viewer.component.css']
})
export class GraphViewerComponent implements OnInit, AfterViewInit {

  simConfig: SimulationConfig;
  @ViewChild(TurnInputComponent) turnInput: TurnInputComponent;

  @ViewChild(ViewInputComponent) viewInput: ViewInputComponent;

  currentZoom = 1;
  currentZoomFormControl: FormControl;

  refreshing: boolean;
  activeSimulation: boolean;

  thumbnail: any;

  constructor(private graphservice: GraphServiceService,
              private dialog: MatDialog) {
  }

  ngOnInit(): void {
    this.currentZoomFormControl = new FormControl(this.currentZoom, [Validators.required]);
    this.currentZoomFormControl.valueChanges.pipe(
      debounceTime(1000),
      distinctUntilChanged()
    ).subscribe(_ => {
      if (this.activeSimulation) {
        this.refreshView();
      }
    });
  }

  ngAfterViewInit(): void {
  }

  openSimulationConfigDialog() {

    const dialogRef = this.dialog.open(SimulationConfiguratorComponent, {
      width: '470px'
    });

    dialogRef.afterClosed().subscribe((data: SimulationConfig) => {
      this.simConfig = data;
      this.graphservice.simulate(this.simConfig).subscribe(response => {
        this.activeSimulation = true;
        this.turnInput.maxTurn = response.end_time;
        this.viewInput.currentCoord.lat = response.view_center[0];
        this.viewInput.currentCoord.lon = response.view_center[1];
        this.viewInput.maxCoord.lat = response.view_bounds.max_lat;
        this.viewInput.minCoord.lat = response.view_bounds.min_lat;
        this.viewInput.maxCoord.lon = response.view_bounds.max_lon;
        this.viewInput.minCoord.lon = response.view_bounds.min_lon;
      });
    });
  }

  public refreshView() {
    if (this.activeSimulation) {
      this.refreshing = true;
      this.graphservice.refreshView(this.turnInput.currentTurn, this.currentZoom, this.viewInput.currentCoord)
        .subscribe((data: Blob) => {
          this.refreshing = false;
          this.createImageFromBlob(data);
        }, _ => {
          console.log('Could not refresh the View');
          this.refreshing = false;
        });
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

  changeZoomBy(value: number) {
    this.currentZoom = Math.round((this.currentZoom + value) * 100) / 100;
  }
}

export enum KEY_CODE {
  UP_ARROW = 'ArrowUp',
  DOWN_ARROW = 'ArrowDown',
  RIGHT_ARROW = 'ArrowRight',
  LEFT_ARROW = 'ArrowLeft'
}
