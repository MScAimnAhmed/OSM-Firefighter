import { Component, ViewChild } from '@angular/core';
import { GraphViewerComponent } from './graph-viewer/graph-viewer.component';

@Component({
  selector: 'app-root',
  templateUrl: './app.component.html',
  styleUrls: ['./app.component.css']
})
export class AppComponent {
  title = 'frontend';

  @ViewChild(GraphViewerComponent) viewer: GraphViewerComponent | undefined;

  transferSimConfig(simConfig: any): void {
    if (this.viewer) {
      this.viewer.startSimulation(simConfig);
    }
  }
}
