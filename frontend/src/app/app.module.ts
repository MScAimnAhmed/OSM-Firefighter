import { NgModule } from '@angular/core';
import { BrowserModule } from '@angular/platform-browser';

import { AppComponent } from './app.component';
import {MatListModule} from "@angular/material/list";
import {MatButtonModule} from "@angular/material/button";
import {MatIconModule} from "@angular/material/icon";
import { SimulationConfiguratorComponent } from './simulation-configurator/simulation-configurator.component';
import {MatDialogModule} from "@angular/material/dialog";
import {MatSelectModule} from "@angular/material/select";
import {MatOptionModule} from "@angular/material/core";
import { BrowserAnimationsModule } from '@angular/platform-browser/animations';
import {MatInputModule} from "@angular/material/input";
import { GraphViewerComponent } from './graph-viewer/graph-viewer.component';
import { HttpClientModule } from '@angular/common/http';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { MatSliderModule } from '@angular/material/slider';
import { TurnInputComponent } from './view-inputs/turn-input/turn-input.component';

@NgModule({
  declarations: [
    AppComponent,
    SimulationConfiguratorComponent,
    GraphViewerComponent,
    TurnInputComponent
  ],
  imports: [
    BrowserModule,
    MatListModule,
    HttpClientModule,
    MatButtonModule,
    MatIconModule,
    MatDialogModule,
    MatSelectModule,
    MatInputModule,
    MatOptionModule,
    BrowserAnimationsModule,
    ReactiveFormsModule,
    MatSliderModule,
    FormsModule
  ],
  providers: [
  ],
  bootstrap: [AppComponent]
})
export class AppModule { }
